fn main() {
    slint_build::compile("src/ui/layer_indicator.slint")
        .expect("could not compile the layer-indicator Slint component");
}
