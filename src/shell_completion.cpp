#include "dcpwizard/shell_completion.h"

#include <sstream>

namespace dcpwizard
{

std::string generate_completion(const std::string& shell)
{
  if (shell == "bash")
  {
    std::ostringstream ss;
    ss << "# bash completion for dcpwizard\n";
    ss << "_dcpwizard() {\n";
    ss << "  local cur prev cmds\n";
    ss << "  COMPREPLY=()\n";
    ss << "  cur=\"${COMP_WORDS[COMP_CWORD]}\"\n";
    ss << "  prev=\"${COMP_WORDS[COMP_CWORD-1]}\"\n";
    ss << "  cmds=\"create encode transcode verify info kdm copy loudness report serve watch completion daemon batch\"\n";
    ss << "  if [[ ${COMP_CWORD} == 1 ]]; then\n";
    ss << "    COMPREPLY=( $(compgen -W \"${cmds}\" -- ${cur}) )\n";
    ss << "    return 0\n";
    ss << "  fi\n";
    ss << "  case \"${prev}\" in\n";
    ss << "    --video|--input|-i|--output|-o)\n";
    ss << "      COMPREPLY=( $(compgen -f -- ${cur}) )\n";
    ss << "      return 0;;\n";
    ss << "    --standard)\n";
    ss << "      COMPREPLY=( $(compgen -W \"smpte interop\" -- ${cur}) )\n";
    ss << "      return 0;;\n";
    ss << "  esac\n";
    ss << "  COMPREPLY=( $(compgen -W \"--title --video --audio --output --standard --profile --encrypt --verbose --help\" -- ${cur}) )\n";
    ss << "}\n";
    ss << "complete -F _dcpwizard dcpwizard\n";
    return ss.str();
  }
  else if (shell == "zsh")
  {
    std::ostringstream ss;
    ss << "#compdef dcpwizard\n";
    ss << "_dcpwizard() {\n";
    ss << "  local -a commands\n";
    ss << "  commands=(\n";
    ss << "    'create:Create a new DCP'\n";
    ss << "    'encode:Encode images to JPEG 2000'\n";
    ss << "    'transcode:Transcode video to image sequence'\n";
    ss << "    'verify:Verify an existing DCP'\n";
    ss << "    'info:Show DCP metadata'\n";
    ss << "    'kdm:Generate KDM'\n";
    ss << "    'copy:Copy DCP to drive'\n";
    ss << "    'loudness:Measure audio loudness'\n";
    ss << "    'report:Generate QC report'\n";
    ss << "    'serve:Start REST API server'\n";
    ss << "    'watch:Watch directory for auto-DCP'\n";
    ss << "  )\n";
    ss << "  _describe 'command' commands\n";
    ss << "}\n";
    ss << "_dcpwizard\n";
    return ss.str();
  }
  else if (shell == "fish")
  {
    std::ostringstream ss;
    ss << "# fish completion for dcpwizard\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a create -d 'Create a new DCP'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a encode -d 'Encode images to JPEG 2000'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a transcode -d 'Transcode video'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a verify -d 'Verify DCP'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a info -d 'Show DCP metadata'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a kdm -d 'Generate KDM'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a copy -d 'Copy DCP to drive'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a loudness -d 'Measure loudness'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a report -d 'Generate QC report'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a serve -d 'Start REST API'\n";
    ss << "complete -c dcpwizard -n '__fish_use_subcommand' -a watch -d 'Watch directory'\n";
    ss << "complete -c dcpwizard -n '__fish_seen_subcommand_from create' -l title -d 'DCP title'\n";
    ss << "complete -c dcpwizard -n '__fish_seen_subcommand_from create' -l video -d 'Video input'\n";
    ss << "complete -c dcpwizard -n '__fish_seen_subcommand_from create' -l audio -d 'Audio WAV'\n";
    ss << "complete -c dcpwizard -n '__fish_seen_subcommand_from create' -l output -d 'Output dir'\n";
    return ss.str();
  }

  return "# Unsupported shell: " + shell + "\n";
}

} // namespace dcpwizard
