use std::{borrow::Cow, collections::HashMap, sync::Arc};

use egui::ahash::{HashSet, HashSetExt};

use crate::{FileChangeWatcher, GraphicsContext};

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
        ShaderSource{
             files: &[$(   $crate::ShaderFile { wgsl: include_str!($file), file: $file, }    ),+]
        }
    }};
}

pub trait HotReload {
    fn source(&self) -> &ShaderSource;
    fn hot_reload(&mut self, shader: &wgpu::ShaderModule);
}

pub struct ShaderCache {
    ctx: GraphicsContext,
    /// maps each file to the current wgsl content.
    current_wgsl: HashMap<ShaderFile, String>,
    module_cache: HashMap<String, std::sync::Weak<wgpu::ShaderModule>>,
    hot_reload_watcher: Option<FileChangeWatcher>,
    hot_reload_shaders_dir: &'static str,
}

impl ShaderCache {
    pub fn new(ctx: GraphicsContext, hot_reload_shaders_dir: Option<&'static str>) -> Self {
        ShaderCache {
            ctx,
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

    pub fn register(&mut self, source: ShaderSource) -> Arc<wgpu::ShaderModule> {
        for file in source.files {
            self.add_file(*file);
        }

        // combine the files into one wgsl string to generate (or get the cached) shader module:
        let mut wgsl = String::new();
        for file in source.files {
            wgsl.push_str(file.wgsl);
        }
        self.get_shader_module(wgsl)
    }

    pub fn hot_reload(&mut self, reload: &mut [&mut dyn HotReload]) {
        let Some(watcher) = &mut self.hot_reload_watcher else {
            return;
        };
        let Some(paths_changed) = watcher.check_for_changes() else {
            return;
        };

        let mut files_to_reload = HashSet::new();
        for p in paths_changed {
            for e in self.current_wgsl.keys() {
                if p.ends_with(e.file) {
                    files_to_reload.insert(*e);
                }
            }
        }

        for f in files_to_reload.iter() {
            let file_path = format!("{}/{}", self.hot_reload_shaders_dir, f.file);
            let content = std::fs::read_to_string(file_path).unwrap();
            self.current_wgsl.insert(*f, content);
        }

        for r in reload {
            let source = r.source();
            let mut wgsl = String::new();
            for f in source.files {
                wgsl.push_str(self.current_wgsl.get(f).unwrap());
            }
            let shader = self.get_shader_module(wgsl);
            r.hot_reload(&shader);
        }
    }

    fn add_file(&mut self, file: ShaderFile) {
        self.current_wgsl.insert(file, file.wgsl.to_owned());
        if let Some(watcher) = &mut self.hot_reload_watcher {
            let file_path = format!("{}/{}", self.hot_reload_shaders_dir, file.file);
            // write a file to e.g. assets/hotreload/ui_rect.wgsl
            std::fs::write(&file_path, file.wgsl).expect("hot_reload_shaders_dir should exist");
            // start watching this file:
            watcher.watch(&file_path);
        }
    }

    fn get_shader_module(&mut self, wgsl: String) -> Arc<wgpu::ShaderModule> {
        if let Some(shader) = self.module_cache.get(&wgsl) {
            if let Some(shader) = shader.upgrade() {
                return shader;
            }
        }

        let shader = self
            .ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&wgsl)),
            });
        let shader = Arc::new(shader);
        self.module_cache.insert(wgsl, Arc::downgrade(&shader));

        shader
    }
}
