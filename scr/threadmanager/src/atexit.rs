pub struct CleanupRegistry {
    funcs: Vec<Box<dyn Fn() + Send + Sync>>,
}

impl CleanupRegistry {
    pub fn new() -> Self {
        CleanupRegistry { funcs: Vec::new() }
    }

    pub fn register<F>(&mut self, func: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.funcs.push(Box::new(func));
    }
}

impl Drop for CleanupRegistry {
    fn drop(&mut self) {
        for func in &self.funcs {
            func();
        }
    }
}