use cargo_metadata::camino::Utf8PathBuf;
use std::{path::PathBuf, sync::atomic::Ordering};
use ui_test::{
    default_any_file_filter, diagnostics::Diagnostics, error_on_output_conflict,
    per_test_config::Comments, run_tests_generic, spanned::Spanned, status_emitter, Args,
    CommandBuilder, Config, Format, Match,
};
use xshell::{cmd, Shell};

fn main() -> ui_test::color_eyre::Result<()> {
    let mut config = config();
    let abort_check = config.abort_check.clone();
    ctrlc::set_handler(move || abort_check.store(true, Ordering::Relaxed))?;

    let args = Args::test()?;
    if let Format::Pretty = args.format {
        println!(
            "Compiler: {}",
            config.program.display().to_string().replace('\\', "/")
        );
    }

    let name = config.root_dir.display().to_string();

    let text = match args.format {
        Format::Terse => status_emitter::Text::quiet(),
        Format::Pretty => status_emitter::Text::verbose(),
    };
    config.with_args(&args);

    run_tests_generic(
        vec![config],
        |path, config| {
            path.extension().filter(|&ext| ext == "ssl")?;
            Some(default_any_file_filter(path, config))
        },
        |_config, _contents| {},
        (text, status_emitter::Gha::<true> { name }),
    )
}

fn config() -> Config {
    let mut comment_defaults = Comments::default();

    let filters = vec![
        (Match::PathBackslash, b"/".to_vec()),
        #[cfg(windows)]
        (Match::Exact(vec![b'\r']), b"".to_vec()),
        #[cfg(windows)]
        (Match::Exact(br"\\?\".to_vec()), b"".to_vec()),
    ];
    comment_defaults
        .base()
        .normalize_stderr
        .clone_from(&filters);
    comment_defaults.base().normalize_stdout = filters;
    comment_defaults.base().exit_status = Spanned::dummy(0).into();
    comment_defaults.base().require_annotations = Spanned::dummy(false).into();
    Config {
        host: Some("irrelevant".into()),
        target: None,
        root_dir: "tests/raw_dumps".into(),
        program: CommandBuilder::cmd(cargo_build("raw_dump")),
        output_conflict_handling: error_on_output_conflict,
        bless_command: Some("cargo test -- -- --bless".into()),
        out_dir: std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap().join("target"))
            .join("ui"),
        skip_files: Vec::new(),
        filter_files: Vec::new(),
        threads: None,
        list: false,
        run_only_ignored: false,
        filter_exact: false,
        comment_defaults,
        comment_start: "//",
        custom_comments: Default::default(),
        diagnostic_extractor: |_, rendered| Diagnostics {
            rendered: rendered.to_owned(),
            messages: vec![],
            messages_from_unknown_file_or_line: vec![],
        },
        abort_check: Default::default(),
    }
}

fn cargo_build(bin: &str) -> Utf8PathBuf {
    let sh = Shell::new().unwrap();
    let output = cmd!(sh, "cargo build --bin {bin} --message-format=json")
        .read()
        .unwrap();
    for line in output.lines() {
        match serde_json::from_str::<cargo_metadata::Message>(line) {
            Ok(cargo_metadata::Message::CompilerArtifact(artifact)) => {
                if artifact.target.name == bin && artifact.target.kind.iter().any(|k| k == "bin") {
                    return artifact.executable.unwrap();
                }
            }
            _ => {}
        }
    }
    panic!("no artifact found for {bin}")
}
