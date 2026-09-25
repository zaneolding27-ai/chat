use std::{
    collections::HashMap,
    fs, io,
    path::{Component, Path, PathBuf},
};

struct CachedResource {
    data: Vec<u8>,
    last_used: u64,
}

pub struct ResourceManager {
    root: PathBuf,
    budget: usize,
    cached_bytes: usize,
    access_counter: u64,
    cache: HashMap<PathBuf, CachedResource>,
}

impl ResourceManager {
    pub fn new(root: impl Into<PathBuf>, budget: usize) -> Self {
        Self {
            root: root.into(),
            budget,
            cached_bytes: 0,
            access_counter: 0,
            cache: HashMap::new(),
        }
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
        let relative_path = Self::validate_path(path.as_ref())?;
        self.access_counter = self.access_counter.wrapping_add(1);
        let access_counter = self.access_counter;

        if let Some(resource) = self.cache.get_mut(&relative_path) {
            resource.last_used = access_counter;
            return Ok(resource.data.clone());
        }

        let data = fs::read(self.root.join(&relative_path))?;
        if data.len() <= self.budget {
            self.cached_bytes += data.len();
            self.cache.insert(
                relative_path,
                CachedResource {
                    data: data.clone(),
                    last_used: access_counter,
                },
            );
            self.evict_until_within_budget();
        }

        Ok(data)
    }

    pub fn cached_bytes(&self) -> usize {
        self.cached_bytes
    }

    pub fn budget(&self) -> usize {
        self.budget
    }

    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        Self::validate_path(path.as_ref())
            .map(|relative_path| self.cache.contains_key(&relative_path))
            .unwrap_or(false)
    }

    fn validate_path(path: &Path) -> io::Result<PathBuf> {
        if path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "resource paths must be relative to the resource directory",
            ));
        }

        let mut relative_path = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => relative_path.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "resource path must stay inside the resource directory",
                    ));
                }
            }
        }

        if relative_path.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "resource path cannot be empty",
            ));
        }
        Ok(relative_path)
    }

    fn evict_until_within_budget(&mut self) {
        while self.cached_bytes > self.budget {
            let Some(oldest_path) = self
                .cache
                .iter()
                .min_by_key(|(_, resource)| resource.last_used)
                .map(|(path, _)| path.clone())
            else {
                break;
            };

            if let Some(resource) = self.cache.remove(&oldest_path) {
                self.cached_bytes -= resource.data.len();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceManager;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn evicts_least_recently_used_resource_when_budget_is_exceeded() {
        let root = std::env::temp_dir().join(format!(
            "ash-vulkan-triangle-resources-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("temporary resource directory should be created");
        fs::write(root.join("a.bin"), b"aaaa").expect("resource a should be written");
        fs::write(root.join("b.bin"), b"bbbb").expect("resource b should be written");
        fs::write(root.join("c.bin"), b"cccc").expect("resource c should be written");

        let mut manager = ResourceManager::new(&root, 8);
        manager.load("a.bin").expect("resource a should load");
        manager.load("b.bin").expect("resource b should load");
        manager
            .load("a.bin")
            .expect("resource a should be a cache hit");
        manager.load("c.bin").expect("resource c should load");

        assert!(manager.contains("a.bin"));
        assert!(!manager.contains("b.bin"));
        assert!(manager.contains("c.bin"));
        assert_eq!(manager.cached_bytes(), manager.budget());

        fs::remove_dir_all(root).expect("temporary resource directory should be removed");
    }
}
