use anyhow::anyhow;
use circular_buffer::CircularBuffer;
use gpui::{AppContext, SharedString, Task, WeakEntity};
use project::ProjectPath;
use ui::{App, IntoElement, Label, ParentElement, Styled, v_flex};
use workspace::{
    Workspace,
    notifications::{NotificationId, simple_message_notification::MessageNotification},
};

const MAX_UNDO_OPERATIONS: usize = 10_000;

pub enum ProjectPanelOperation {
    Batch(Vec<ProjectPanelOperation>),
    Create {
        project_path: ProjectPath,
    },
    Rename {
        old_path: ProjectPath,
        new_path: ProjectPath,
    },
}

impl ProjectPanelOperation {
    pub fn batch(operations: impl IntoIterator<Item = Self>) -> Option<Self> {
        let mut operations: Vec<_> = operations.into_iter().collect();
        match operations.len() {
            0 => None,
            1 => operations.pop(),
            _ => Some(Self::Batch(operations)),
        }
    }
}

pub struct UndoManager {
    workspace: WeakEntity<Workspace>,
    stack: Box<CircularBuffer<MAX_UNDO_OPERATIONS, ProjectPanelOperation>>,
}

impl UndoManager {
    pub fn new(workspace: WeakEntity<Workspace>) -> Self {
        Self {
            workspace,
            stack: CircularBuffer::boxed(),
        }
    }

    pub fn undo(&mut self, cx: &mut App) {
        if let Some(operation) = self.stack.pop_back() {
            let task = self.revert_operation(operation, cx);
            let workspace = self.workspace.clone();

            cx.spawn(async move |cx| {
                let errors = task.await;
                if !errors.is_empty() {
                    cx.update(|cx| {
                        let messages = errors
                            .iter()
                            .map(|err| SharedString::from(err.to_string()))
                            .collect();

                        Self::show_errors(workspace, messages, cx)
                    })
                }
            })
            .detach();
        }
    }

    pub fn record(&mut self, operations: impl IntoIterator<Item = ProjectPanelOperation>) {
        if let Some(operation) = ProjectPanelOperation::batch(operations) {
            self.stack.push_back(operation);
        }
    }

    /// Attempts to revert the provided `operation`, returning a vector of errors
    /// in case there was any failure while reverting the operation.
    ///
    /// For all operations other than [`crate::undo::ProjectPanelOperation::Batch`], a maximum
    /// of one error is returned.
    fn revert_operation(
        &self,
        operation: ProjectPanelOperation,
        cx: &mut App,
    ) -> Task<Vec<anyhow::Error>> {
        match operation {
            ProjectPanelOperation::Create { project_path } => {
                let Some(workspace) = self.workspace.upgrade() else {
                    return Task::ready(vec![anyhow!("Failed to obtain workspace.")]);
                };

                let result = workspace.update(cx, |workspace, cx| {
                    workspace.project().update(cx, |project, cx| {
                        let entry_id = project
                            .entry_for_path(&project_path, cx)
                            .map(|entry| entry.id)
                            .ok_or_else(|| anyhow!("No entry for path."))?;

                        project
                            .delete_entry(entry_id, true, cx)
                            .ok_or_else(|| anyhow!("Failed to trash entry."))
                    })
                });

                let task = match result {
                    Ok(task) => task,
                    Err(err) => return Task::ready(vec![err]),
                };

                cx.spawn(async move |_| match task.await {
                    Ok(_) => vec![],
                    Err(err) => vec![err],
                })
            }
            ProjectPanelOperation::Rename { old_path, new_path } => {
                let Some(workspace) = self.workspace.upgrade() else {
                    return Task::ready(vec![anyhow!("Failed to obtain workspace.")]);
                };

                let result = workspace.update(cx, |workspace, cx| {
                    workspace.project().update(cx, |project, cx| {
                        let entry_id = project
                            .entry_for_path(&new_path, cx)
                            .map(|entry| entry.id)
                            .ok_or_else(|| anyhow!("No entry for path."))?;

                        Ok(project.rename_entry(entry_id, old_path.clone(), cx))
                    })
                });

                let task = match result {
                    Ok(task) => task,
                    Err(err) => return Task::ready(vec![err]),
                };

                cx.spawn(async move |_| match task.await {
                    Ok(_) => vec![],
                    Err(err) => vec![err],
                })
            }
            ProjectPanelOperation::Batch(operations) => {
                let tasks: Vec<_> = operations
                    .into_iter()
                    .map(|op| self.revert_operation(op, cx))
                    .collect();

                // TODO!: Update to use a sequential approach instead of
                // parallel tasks, and to collect the errors so we can display
                // all in an error message.
                cx.spawn(async move |_| {
                    let results = futures::future::join_all(tasks).await;
                    let errors: Vec<_> = results
                        .into_iter()
                        .filter(|errors| !errors.is_empty())
                        .flatten()
                        .collect();

                    if errors.is_empty() { vec![] } else { errors }
                })
            }
        }
    }

    /// Displays a notification with the list of provided errors ensuring that,
    /// when more than one error is provided, which can be the case when dealing
    /// with undoing a [`crate::undo::ProjectPanelOperation::Batch`], a list is
    /// displayed with each of the errors, instead of a single message.
    fn show_errors(workspace: WeakEntity<Workspace>, messages: Vec<SharedString>, cx: &mut App) {
        workspace
            .update(cx, move |workspace, cx| {
                let notification_id =
                    NotificationId::Named(SharedString::new_static("project_panel_undo"));

                workspace.show_notification(notification_id, cx, move |cx| {
                    let messages = messages.clone();

                    cx.new(|cx| {
                        if let [err] = messages.as_slice() {
                            MessageNotification::new(err.to_string(), cx)
                                .with_title("Failed to undo Project Panel Operation")
                        } else {
                            MessageNotification::new_from_builder(cx, move |_, _| {
                                v_flex()
                                    .gap_1()
                                    .children(
                                        messages
                                            .iter()
                                            .map(|message| Label::new(format!("- {message}"))),
                                    )
                                    .into_any_element()
                            })
                            .with_title("Failed to undo Project Panel Operations")
                        }
                    })
                })
            })
            .ok();
    }
}
