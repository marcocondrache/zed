use std::any::Any;

pub struct AnyVec {
    inner: Box<dyn Any + Send>,
    len: usize,
}

impl AnyVec {
    pub fn new<T: Send + 'static>() -> Self {
        Self {
            inner: Box::new(Vec::<T>::new()),
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn at(&self, index: usize) -> AnyElementRef<'_> {
        AnyElementRef { vec: self, index }
    }

    pub fn get<T: Send + 'static>(&self, index: usize) -> Option<&T> {
        self.downcast_ref().and_then(|vec| vec.get(index))
    }

    pub fn iter<T: Send + 'static>(&self) -> Option<std::slice::Iter<'_, T>> {
        self.downcast_ref().map(|vec| vec.iter())
    }

    pub fn iter_mut<T: Send + 'static>(&mut self) -> Option<std::slice::IterMut<'_, T>> {
        self.downcast_mut().map(|vec| vec.iter_mut())
    }

    pub fn as_slice<T: Send + 'static>(&self) -> Option<&[T]> {
        self.downcast_ref().map(|vec| vec.as_slice())
    }

    pub fn as_mut_slice<T: Send + 'static>(&mut self) -> Option<&mut [T]> {
        self.downcast_mut().map(|vec| vec.as_mut_slice())
    }

    pub fn try_clone<T: Clone + Send + 'static>(&self) -> Option<Self> {
        self.downcast_ref()
            .map(|vec: &Vec<T>| vec.clone())
            .map(Self::from)
    }

    fn downcast_ref<T: Send + 'static>(&self) -> Option<&Vec<T>> {
        self.inner.downcast_ref()
    }

    fn downcast_mut<T: Send + 'static>(&mut self) -> Option<&mut Vec<T>> {
        self.inner.downcast_mut()
    }
}

impl<T: Send + 'static> From<Vec<T>> for AnyVec {
    fn from(value: Vec<T>) -> Self {
        Self {
            len: value.len(),
            inner: Box::new(value),
        }
    }
}

pub struct AnyElementRef<'a> {
    vec: &'a AnyVec,
    index: usize,
}

impl<'a> AnyElementRef<'a> {
    pub fn downcast_ref<T: Send + 'static>(&self) -> Option<&T> {
        self.vec
            .downcast_ref()
            .and_then(|inner| inner.get(self.index))
    }
}
