use clap::parser::ValueSource;
use clap::{Arg, ArgMatches, Command};
use dcpwizard_core::presets::Preset;
use serde_json::Value;
use std::ffi::OsString;

pub const PRESET_ARGUMENT: &str = "preset";

// the panel choices that mean the flag is left off, beside an empty field
const ANY_TEXT: &str = "";
const AUTOMATIC_CHOICE: &str = "auto";
const OFF_CHOICE: &str = "none";
const SINGLE_REEL_CHOICE: &str = "0";

const AUDIO_MAP_ARGUMENT: &str = "audio_map";
const CONTENT_TYPE_ARGUMENT: &str = "content_type";
const LUMINANCE_ARGUMENT: &str = "luminance";
const LUMINANCE_UNITS_KEY: &str = "luminanceUnits";

enum CreateOption {
    // the field's text follows the flag unless it is empty or the left-off choice
    Text {
        argument: &'static str,
        left_off_choice: &'static str,
    },
    Ticked(&'static str),
    Unticked(&'static str),
    // the panel names the CPL kind where create takes the abbreviation
    ContentKind,
    // create takes the number and its unit as one value
    Luminance,
    LuminanceUnits,
}

const fn text(argument: &'static str) -> CreateOption {
    CreateOption::Text {
        argument,
        left_off_choice: ANY_TEXT,
    }
}

const fn text_unless(argument: &'static str, left_off_choice: &'static str) -> CreateOption {
    CreateOption::Text {
        argument,
        left_off_choice,
    }
}

// keyed like FORM_CONTROLS in the GUI's project-form.js
const CREATE_OPTION_BY_FORM_KEY: &[(&str, CreateOption)] = &[
    ("validate", CreateOption::Unticked("no_verify")),
    ("standard", text("standard")),
    ("resolution", text_unless("container", AUTOMATIC_CHOICE)),
    ("framerate", text_unless("frame_rate", AUTOMATIC_CHOICE)),
    ("bandwidth", text("video_bit_rate")),
    ("qualityPsnr", text("quality_psnr")),
    ("contentKind", CreateOption::ContentKind),
    ("encrypt", CreateOption::Ticked("encrypt")),
    ("subtitleLanguage", text("subtitle_language")),
    ("subtitleFontSize", text("subtitle_font_size")),
    ("subtitleColour", text("subtitle_colour")),
    ("subtitleEffect", text("subtitle_effect")),
    ("subtitleEffectColour", text("subtitle_effect_colour")),
    ("subtitleFadeUp", text("subtitle_fade_up")),
    ("subtitleFadeDown", text("subtitle_fade_down")),
    ("subtitleHalign", text("subtitle_halign")),
    ("subtitleValign", text("subtitle_valign")),
    ("subtitleVposition", text("subtitle_vposition")),
    ("subtitleZposition", text("subtitle_zposition")),
    ("subtitleRtl", text("subtitle_rtl")),
    ("subtitleWrap", text("subtitle_wrap")),
    ("subtitleFont", text("subtitle_font")),
    (
        "subtitleNoSubset",
        CreateOption::Ticked("subtitle_no_subset"),
    ),
    ("burnSubtitleFont", text("burn_subtitle_font")),
    ("burnFontSize", text("burn_font_size")),
    ("burnColour", text("burn_colour")),
    ("burnEffect", text("burn_effect")),
    ("burnEffectColour", text("burn_effect_colour")),
    ("burnOutlineWidth", text("burn_outline_width")),
    ("burnLineHeight", text("burn_line_height")),
    ("burnMargin", text("burn_margin")),
    ("burnFadeUp", text("burn_fade_up")),
    ("burnFadeDown", text("burn_fade_down")),
    ("ccapLanguage", text("ccap_language")),
    ("loudnessTarget", text("loudness_target")),
    ("truePeakCeiling", text("true_peak_ceiling")),
    ("audioGainDb", text("audio_gain")),
    ("audioFadeInSeconds", text("audio_fade_in")),
    ("audioFadeOutSeconds", text("audio_fade_out")),
    ("audioInputOrder", text("audio_input_order")),
    (
        "audioChannels",
        text_unless("audio_channels", AUTOMATIC_CHOICE),
    ),
    ("signLanguageTag", text("sign_language_lang")),
    ("padHead", text("pad_head")),
    ("padTail", text("pad_tail")),
    ("padColor", text("pad_color")),
    ("audioDelayMs", text("audio_delay")),
    ("videoFadeInSeconds", text("video_fade_in")),
    ("videoFadeOutSeconds", text("video_fade_out")),
    ("sourceColourspace", text("source_colourspace")),
    ("cropLeft", text("crop_left")),
    ("cropRight", text("crop_right")),
    ("cropTop", text("crop_top")),
    ("cropBottom", text("crop_bottom")),
    ("fillCrop", CreateOption::Ticked("fill_crop")),
    ("deinterlace", CreateOption::Ticked("deinterlace")),
    ("denoise", CreateOption::Ticked("denoise")),
    ("rotate", text_unless("rotate", OFF_CHOICE)),
    ("flip", text_unless("flip", OFF_CHOICE)),
    ("upmix", text_unless("upmix", OFF_CHOICE)),
    (
        "reelLengthMinutes",
        text_unless("reel_length", SINGLE_REEL_CHOICE),
    ),
    ("splitChapters", CreateOption::Ticked("split_chapters")),
    ("hdrDci", CreateOption::Ticked("hdr_dci")),
    ("hdrSource", text_unless("hdr_source", AUTOMATIC_CHOICE)),
    ("hdrPeakNits", text("hdr_peak_nits")),
    ("hdrToDciLut", text("hdr_to_dci_lut")),
    ("hdrAlreadyPq", CreateOption::Ticked("hdr_already_pq")),
    (
        "allowGenericHdrTonemap",
        CreateOption::Ticked("allow_generic_hdr_tonemap"),
    ),
    ("audioLanguage", text("audio_lang")),
    ("studio", text("studio")),
    ("territoryType", text("territory_type")),
    ("chain", text("chain")),
    ("distributor", text("distributor")),
    ("facilityName", text("facility")),
    ("luminance", CreateOption::Luminance),
    (LUMINANCE_UNITS_KEY, CreateOption::LuminanceUnits),
];

#[derive(Debug, PartialEq)]
pub struct PresetArguments {
    pub arguments: Vec<String>,
    pub keys_without_option: Vec<String>,
}

fn field_text(preset: &Preset, key: &str, value: &Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.trim().to_string()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Null => Ok(String::new()),
        _ => Err(format!(
            "preset {}: {key} holds {value}, where a text field was expected",
            preset.name
        )),
    }
}

fn field_ticked(preset: &Preset, key: &str, value: &Value) -> Result<bool, String> {
    value.as_bool().ok_or_else(|| {
        format!(
            "preset {}: {key} holds {value}, where a tick box was expected",
            preset.name
        )
    })
}

fn luminance_value(preset: &Preset, number: &str) -> Result<String, String> {
    let units = match preset.form.get(LUMINANCE_UNITS_KEY) {
        Some(units) => field_text(preset, LUMINANCE_UNITS_KEY, units)?,
        None => String::new(),
    };
    Ok(format!("{number} {units}").trim().to_string())
}

// the create argument and its value, None when the preset leaves the flag off
fn create_value(
    preset: &Preset,
    key: &str,
    option: &CreateOption,
    value: &Value,
) -> Result<Option<(&'static str, Option<String>)>, String> {
    let given_text = |left_off_choice: &str| -> Result<Option<String>, String> {
        let text = field_text(preset, key, value)?;
        Ok((!text.is_empty() && text != left_off_choice).then_some(text))
    };
    Ok(match option {
        CreateOption::Text {
            argument,
            left_off_choice,
        } => given_text(left_off_choice)?.map(|text| (*argument, Some(text))),
        CreateOption::Ticked(argument) => {
            field_ticked(preset, key, value)?.then_some((*argument, None))
        }
        CreateOption::Unticked(argument) => {
            (!field_ticked(preset, key, value)?).then_some((*argument, None))
        }
        CreateOption::ContentKind => match given_text(ANY_TEXT)? {
            Some(kind) => {
                let abbreviation = dcpwizard_core::ContentType::abbreviation_of_cpl_kind(&kind)
                    .ok_or_else(|| {
                        format!(
                            "preset {}: {key} names no content kind: {kind}",
                            preset.name
                        )
                    })?;
                Some((CONTENT_TYPE_ARGUMENT, Some(abbreviation.to_string())))
            }
            None => None,
        },
        CreateOption::Luminance => match given_text(ANY_TEXT)? {
            Some(number) => Some((LUMINANCE_ARGUMENT, Some(luminance_value(preset, &number)?))),
            None => None,
        },
        CreateOption::LuminanceUnits => None,
    })
}

fn create_argument<'a>(create: &'a Command, id: &str) -> &'a Arg {
    create
        .get_arguments()
        .find(|argument| argument.get_id() == id)
        .unwrap_or_else(|| panic!("create has no {id} argument"))
}

fn given_on_command_line(given: &ArgMatches, id: &str) -> bool {
    given.value_source(id) == Some(ValueSource::CommandLine)
}

fn declares_conflict(create: &Command, argument: &Arg, other: &Arg) -> bool {
    create
        .get_arg_conflicts_with(argument)
        .iter()
        .any(|conflicting| conflicting.get_id() == other.get_id())
}

// a flag given, or one that rules the preset's flag out, wins over the preset
fn overridden(create: &Command, given: &ArgMatches, argument: &Arg) -> bool {
    given_on_command_line(given, argument.get_id().as_str())
        || create
            .get_arguments()
            .filter(|other| given_on_command_line(given, other.get_id().as_str()))
            .any(|other| {
                declares_conflict(create, argument, other)
                    || declares_conflict(create, other, argument)
            })
}

fn command_line_argument(argument: &Arg, value: Option<String>) -> String {
    let long = argument
        .get_long()
        .expect("every preset argument is a long flag");
    match value {
        Some(value) => format!("--{long}={value}"),
        None => format!("--{long}"),
    }
}

pub fn preset_arguments(
    preset: &Preset,
    create: &Command,
    given: &ArgMatches,
) -> Result<PresetArguments, String> {
    let mut values = Vec::new();
    let mut keys_without_option = Vec::new();
    for (key, value) in &preset.form {
        let Some((_, option)) = CREATE_OPTION_BY_FORM_KEY
            .iter()
            .find(|(form_key, _)| form_key == key)
        else {
            keys_without_option.push(key.clone());
            continue;
        };
        values.extend(create_value(preset, key, option, value)?);
    }
    if let Some(audio_map) = preset.audio_map.as_deref().filter(|map| !map.is_empty()) {
        values.push((AUDIO_MAP_ARGUMENT, Some(audio_map.to_string())));
    }
    let arguments = values
        .into_iter()
        .map(|(id, value)| (create_argument(create, id), value))
        .filter(|(argument, _)| !overridden(create, given, argument))
        .map(|(argument, value)| command_line_argument(argument, value))
        .collect();
    Ok(PresetArguments {
        arguments,
        keys_without_option,
    })
}

fn saved_preset(name: &str) -> Result<Preset, String> {
    let path = dcpwizard_core::presets::presets_path();
    let presets =
        dcpwizard_core::presets::load_presets(&path).map_err(|error| error.to_string())?;
    if presets.is_empty() {
        return Err(format!(
            "unknown preset '{name}': no presets are saved in {}",
            path.display()
        ));
    }
    let names: Vec<&str> = presets.iter().map(|preset| preset.name.as_str()).collect();
    let unknown = format!("unknown preset '{name}'. Saved: {}", names.join(", "));
    presets
        .iter()
        .find(|preset| preset.name == name)
        .cloned()
        .ok_or(unknown)
}

// create parses again with the preset's flags after the ones given
pub fn with_preset_arguments(
    matches: ArgMatches,
    command: Command,
    create_subcommand: &str,
) -> ArgMatches {
    let Some(given) = matches.subcommand_matches(create_subcommand) else {
        return matches;
    };
    let Some(name) = given.get_one::<String>(PRESET_ARGUMENT) else {
        return matches;
    };
    let create = command
        .find_subcommand(create_subcommand)
        .expect("create is a subcommand of the CLI");
    let expanded = saved_preset(name).and_then(|preset| preset_arguments(&preset, create, given));
    let expanded = expanded.unwrap_or_else(|error| {
        eprintln!("error: {error}");
        std::process::exit(1);
    });
    for key in &expanded.keys_without_option {
        eprintln!("warning: preset {name}: {key} has no create option and is ignored");
    }
    let arguments = std::env::args_os().chain(expanded.arguments.into_iter().map(OsString::from));
    command.get_matches_from(arguments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use serde_json::json;
    use std::collections::BTreeMap;

    const CREATE: &str = "create";
    const REQUIRED: [&str; 7] = [
        "dcpwizard",
        CREATE,
        "--title",
        "Film",
        "--video",
        "film.mov",
        "--output=/out",
    ];

    fn preset(form: Value, audio_map: Option<&str>) -> Preset {
        let form: BTreeMap<String, Value> = serde_json::from_value(form).unwrap();
        Preset {
            name: "Festival".to_string(),
            form,
            audio_map: audio_map.map(str::to_string),
        }
    }

    // the debug build of create's parser overflows a test thread's stack
    fn on_main_thread_stack<T: Send + 'static>(run: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(crate::MAIN_THREAD_STACK_BYTES)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap()
    }

    fn arguments_for(
        preset: Preset,
        given: &'static [&'static str],
    ) -> Result<PresetArguments, String> {
        on_main_thread_stack(move || {
            let command = crate::Cli::command();
            let matches = command
                .clone()
                .try_get_matches_from(REQUIRED.iter().chain(given))
                .unwrap();
            let create = command.find_subcommand(CREATE).unwrap();
            preset_arguments(&preset, create, matches.subcommand_matches(CREATE).unwrap())
        })
    }

    #[test]
    fn every_option_in_the_table_is_a_create_flag() {
        on_main_thread_stack(assert_every_option_is_a_create_flag);
    }

    fn assert_every_option_is_a_create_flag() {
        let command = crate::Cli::command();
        let create = command.find_subcommand(CREATE).unwrap();
        let mut ids: Vec<&str> = vec![
            AUDIO_MAP_ARGUMENT,
            CONTENT_TYPE_ARGUMENT,
            LUMINANCE_ARGUMENT,
        ];
        for (_, option) in CREATE_OPTION_BY_FORM_KEY {
            match option {
                CreateOption::Text { argument, .. }
                | CreateOption::Ticked(argument)
                | CreateOption::Unticked(argument) => ids.push(argument),
                _ => {}
            }
        }
        for id in ids {
            assert!(create_argument(create, id).get_long().is_some(), "{id}");
        }
    }

    #[test]
    fn panel_values_become_create_flags() {
        let festival = preset(
            json!({
                "standard": "interop",
                "resolution": "2k-flat",
                "framerate": "auto",
                "contentKind": "trailer",
                "validate": false,
                "encrypt": true,
                "denoise": false,
                "audioGainDb": "-6",
                "rotate": "none",
                "luminance": "14",
                "luminanceUnits": "foot-lambert",
                "subtitleFont": "",
            }),
            Some("1:L,2:R@-3"),
        );

        let mut arguments = arguments_for(festival, &[]).unwrap().arguments;
        arguments.sort();

        assert_eq!(
            arguments,
            vec![
                "--audio-gain=-6",
                "--audio-map=1:L,2:R@-3",
                "--container=2k-flat",
                "--content-type=TLR",
                "--encrypt",
                "--luminance=14 foot-lambert",
                "--no-verify",
                "--standard=interop",
            ]
        );
    }

    #[test]
    fn a_given_flag_and_one_that_rules_the_preset_flag_out_both_win() {
        let festival = preset(
            json!({"standard": "interop", "resolution": "2k-flat", "bandwidth": "200"}),
            None,
        );

        let arguments = arguments_for(
            festival,
            &["--standard", "smpte", "--container-dims", "1920x1080"],
        )
        .unwrap()
        .arguments;

        assert_eq!(arguments, vec!["--video-bit-rate=200"]);
    }

    #[test]
    fn a_key_with_no_create_option_is_named() {
        let festival = preset(json!({"standard": "interop", "someLaterField": "3"}), None);

        let expanded = arguments_for(festival, &[]).unwrap();

        assert_eq!(expanded.keys_without_option, vec!["someLaterField"]);
        assert_eq!(expanded.arguments, vec!["--standard=interop"]);
    }

    #[test]
    fn a_text_value_in_a_tick_box_is_refused() {
        let festival = preset(json!({"encrypt": "yes"}), None);

        let error = arguments_for(festival, &[]).unwrap_err();

        assert!(error.contains("encrypt"), "{error}");
    }
}
