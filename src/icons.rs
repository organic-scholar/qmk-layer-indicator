use std::borrow::Cow;

static ICONS: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/assets/icons");

pub fn contains(name: &str) -> bool {
    ICONS.get_file(format!("{name}.svg")).is_some()
}

pub fn image_source(name: &str) -> Option<egui::ImageSource<'static>> {
    let file = ICONS.get_file(format!("{name}.svg"))?;
    Some(egui::ImageSource::Bytes {
        uri: Cow::Owned(format!("bytes://assets/icons/{name}.svg")),
        bytes: egui::load::Bytes::Static(file.contents()),
    })
}
