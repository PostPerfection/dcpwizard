/// Generate shell completion scripts for bash, zsh, and fish.
///
/// Generates completion scripts that can be sourced in the user's shell.
pub fn generate_completion(shell: &str, binary_name: &str) -> String {
    match shell.to_lowercase().as_str() {
        "bash" => generate_bash_completion(binary_name),
        "zsh" => generate_zsh_completion(binary_name),
        "fish" => generate_fish_completion(binary_name),
        _ => {
            tracing::error!("Unsupported shell: {shell}. Use bash, zsh, or fish.");
            String::new()
        }
    }
}

fn generate_bash_completion(name: &str) -> String {
    let subcommands =
        "create verify export import encode wrap info qc report copy watch serve preferences";
    let create_opts =
        "--title --standard --resolution --frame-rate --bitrate --encrypt --output-dir";
    let verify_opts = "--verbose --strict";
    let export_opts = "--format --quality --output";
    let import_opts = "--fps --width --height --format --output-dir";

    format!(
        r#"_{name}_completions() {{
    local cur prev subcommands
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"
    subcommands="{subcommands}"

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=($(compgen -W "$subcommands" -- "$cur"))
        return
    fi

    case "${{COMP_WORDS[1]}}" in
        create)
            COMPREPLY=($(compgen -W "{create_opts}" -- "$cur"))
            ;;
        verify)
            COMPREPLY=($(compgen -W "{verify_opts}" -- "$cur"))
            ;;
        export)
            COMPREPLY=($(compgen -W "{export_opts}" -- "$cur"))
            ;;
        import)
            COMPREPLY=($(compgen -W "{import_opts}" -- "$cur"))
            ;;
        *)
            COMPREPLY=($(compgen -f -- "$cur"))
            ;;
    esac
}}

complete -F _{name}_completions {name}
"#
    )
}

fn generate_zsh_completion(name: &str) -> String {
    format!(
        r#"#compdef {name}

_arguments \
    '1:command:(create verify export import encode wrap info qc report copy watch serve preferences)' \
    '*::arg:->args'

case $state in
    args)
        case $words[1] in
            create)
                _arguments \
                    '--title[DCP title]:title:' \
                    '--standard[DCI standard]:(smpte interop)' \
                    '--resolution[Resolution]:(2k 4k)' \
                    '--frame-rate[Frame rate]:fps:(24 25 30 48 60)' \
                    '--bitrate[Max bitrate in Mbps]:mbps:' \
                    '--encrypt[Enable encryption]' \
                    '--output-dir[Output directory]:dir:_directories'
                ;;
            verify)
                _arguments \
                    '--verbose[Verbose output]' \
                    '--strict[Strict mode]' \
                    '*:dcp directory:_directories'
                ;;
            export)
                _arguments \
                    '--format[Output format]:(prores h264 h265 dnxhr image-sequence)' \
                    '--quality[CRF quality]:crf:' \
                    '--output[Output file]:file:_files'
                ;;
            import)
                _arguments \
                    '--fps[Target frame rate]:fps:(24 25 30 48 60)' \
                    '--width[Target width]:width:' \
                    '--height[Target height]:height:' \
                    '--format[Image format]:(tiff dpx exr png)' \
                    '--output-dir[Output directory]:dir:_directories' \
                    '*:input file:_files'
                ;;
            *)
                _files
                ;;
        esac
        ;;
esac
"#
    )
}

fn generate_fish_completion(name: &str) -> String {
    let mut out = String::new();

    // Subcommands
    let subcommands = [
        ("create", "Create a new DCP"),
        ("verify", "Verify an existing DCP"),
        ("export", "Export DCP to delivery format"),
        ("import", "Import video for DCP creation"),
        ("encode", "Encode images to JPEG 2000"),
        ("wrap", "Wrap essence into MXF"),
        ("info", "Show DCP metadata"),
        ("qc", "Run quality control checks"),
        ("report", "Generate QC report"),
        ("copy", "Copy DCP to drive"),
        ("watch", "Watch directory for new DCPs"),
        ("serve", "Start REST API server"),
        ("preferences", "Manage preferences"),
    ];

    for (cmd, desc) in &subcommands {
        out.push_str(&format!(
            "complete -c {name} -n '__fish_use_subcommand' -a {cmd} -d '{desc}'\n"
        ));
    }

    // Create subcommand options
    let create_opts = [
        ("title", "DCP title"),
        ("standard", "DCI standard (smpte/interop)"),
        ("resolution", "Resolution (2k/4k)"),
        ("frame-rate", "Frame rate"),
        ("bitrate", "Max bitrate in Mbps"),
        ("output-dir", "Output directory"),
    ];

    for (opt, desc) in &create_opts {
        out.push_str(&format!(
            "complete -c {name} -n '__fish_seen_subcommand_from create' -l {opt} -d '{desc}'\n"
        ));
    }
    out.push_str(&format!(
        "complete -c {name} -n '__fish_seen_subcommand_from create' -l encrypt -d 'Enable encryption'\n"
    ));

    // Export subcommand options
    out.push_str(&format!(
        "complete -c {name} -n '__fish_seen_subcommand_from export' -l format -d 'Output format' -xa 'prores h264 h265 dnxhr image-sequence'\n"
    ));
    out.push_str(&format!(
        "complete -c {name} -n '__fish_seen_subcommand_from export' -l quality -d 'CRF quality'\n"
    ));
    out.push_str(&format!(
        "complete -c {name} -n '__fish_seen_subcommand_from export' -l output -d 'Output file'\n"
    ));

    out
}
