//! plove-core — ядро plove.
//!
//! Здесь живёт вся логика, а CLI (и будущие TUI, GUI, ИИ) только вызывают
//! эти функции. Это как бизнес-слой в C#-приложении: контроллеры тонкие,
//! вся работа в сервисах.
//!
//! Путь данных через ядро:
//!
//! ```text
//! rice.yaml ──► spec (разбор и проверка)
//!           ──► plan (подбор модулей, рендер конфигов, сравнение с диском)
//!           ──► apply (пакеты, снимок старых файлов, запись, перезагрузка)
//!           ──► state (история применений и откат)
//! ```

// `pub mod x;` подключает файл `x.rs` как модуль — примерно как
// `import x` в Python или отдельный namespace в C#.
pub mod apply;
pub mod check;
pub mod modules;
pub mod plan;
pub mod presets;
pub mod render;
pub mod spec;
pub mod state;
pub mod system;
pub mod theme;
pub mod wallpaper;
