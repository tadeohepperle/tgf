use std::{borrow::Cow, collections::HashMap, sync::Arc};

use egui::ahash::{HashSet, HashSetExt};

use crate::FileChangeWatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderFile {
    pub file: &'static str,
    pub wgsl: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderSource {
    pub files: &'static [ShaderFile],
}

impl ShaderSource {}

#[macro_export]
macro_rules! make_shader_source {
    ($($file:literal),+) => {{
        $crate::ShaderSource{
             files: &[$(   $crate::ShaderFile { wgsl: include_str!($file), file: $file, }    ),+]
        }
    }};
}

pub trait HotReload {
    fn source(&self) -> ShaderSource;
    fn hot_reload(&mut self, shader: &wgpu::ShaderModule, device: &wgpu::Device);
}

#[derive(Debug)]
pub struct ShaderCache {
    /// maps each file to the current wgsl content.
    current_wgsl: HashMap<ShaderFile, String>,
    module_cache: HashMap<String, std::sync::Weak<wgpu::ShaderModule>>,
    hot_reload_watcher: Option<FileChangeWatcher>,
    hot_reload_shaders_dir: &'static str,
}

impl ShaderCache {
    pub fn new(hot_reload_shaders_dir: Option<&'static str>) -> Self {
        ShaderCache {
            current_wgsl: HashMap::new(),
            module_cache: HashMap::new(),
            hot_reload_watcher: if let Some(dir) = hot_reload_shaders_dir {
                std::fs::create_dir_all(dir).unwrap();
                Some(FileChangeWatcher::new(&[]))
            } else {
                None
            },
            hot_reload_shaders_dir: hot_reload_shaders_dir.unwrap_or("no_hot_reload"),
        }
    }

    pub fn register(
        &mut self,
        source: ShaderSource,
        device: &wgpu::Device,
    ) -> Arc<wgpu::ShaderModule> {
        for file in source.files {
            self.add_file(*file);
        }

        // combine the files into one wgsl string to generate (or get the cached) shader module:
        let mut wgsl = String::new();
        for f in source.files {
            wgsl.push_str(self.current_wgsl.get(f).unwrap());
        }
        if let Err(err) = validate_wgsl(&wgsl) {
            panic!("Error: {err}");
        }
        self.get_shader_module(wgsl, device)
    }

    /// checks for changes in the watched paths and if so, updates all the hotreloadable renderers.
    pub fn hot_reload(&mut self, reload: &mut [&mut dyn HotReload], device: &wgpu::Device) {
        let Some(watcher) = &mut self.hot_reload_watcher else {
            return;
        };
        let Some(paths_changed) = watcher.check_for_changes() else {
            return;
        };

        dbg!(&paths_changed);

        let mut files_to_reload = HashSet::new();
        for p in paths_changed {
            for e in self.current_wgsl.keys() {
                if p.ends_with(e.file) {
                    files_to_reload.insert(*e);
                }
            }
        }

        for f in files_to_reload.iter() {
            let path = format!("{}/{}", self.hot_reload_shaders_dir, f.file);
            if std::path::Path::new(&path).exists() {
                let wgsl = std::fs::read_to_string(&path).unwrap();
                self.current_wgsl.insert(*f, wgsl);
            }
        }

        dbg!(reload.len());
        for r in reload {
            let source = r.source();

            let mut wgsl = String::new();
            for f in source.files {
                wgsl.push_str(self.current_wgsl.get(f).unwrap());
            }

            if let Err(err) = validate_wgsl(&wgsl) {
                println!("Hot-Reload-Error: {err}");
            } else {
                let shader = self.get_shader_module(wgsl, device);
                r.hot_reload(&shader, device);
            }
        }
    }

    fn add_file(&mut self, file: ShaderFile) {
        let wgsl: String;
        if let Some(watcher) = &mut self.hot_reload_watcher {
            let path = format!("{}/{}", self.hot_reload_shaders_dir, file.file);

            if std::path::Path::new(&path).exists() {
                wgsl = std::fs::read_to_string(&path).unwrap();
            } else {
                wgsl = file.wgsl.to_owned();
                std::fs::write(&path, &file.wgsl).unwrap();
            }
            watcher.watch(&path);
        } else {
            wgsl = file.wgsl.to_owned();
        }

        self.current_wgsl.insert(file, wgsl);
    }

    fn get_shader_module(
        &mut self,
        wgsl: String,
        device: &wgpu::Device,
    ) -> Arc<wgpu::ShaderModule> {
        if let Some(shader) = self.module_cache.get(&wgsl) {
            if let Some(shader) = shader.upgrade() {
                return shader;
            }
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&wgsl)),
        });
        let shader = Arc::new(shader);
        self.module_cache.insert(wgsl, Arc::downgrade(&shader));

        shader
    }
}

fn validate_wgsl(wgsl: &str) -> anyhow::Result<()> {
    wgpu::naga::front::wgsl::parse_str(&wgsl)?;
    Ok(())
}
