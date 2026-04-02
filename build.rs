fn main() {
    slint_build::compile_with_config(
        "src/ui/gesture_app.slint",
        slint_build::CompilerConfiguration::new()
            .with_style("fluent-light".to_string()),
    )
    .expect("Slint build failed");
}
