#pragma once

#include <string>

namespace dcpwizard
{

/// Generate shell completion scripts (bash, zsh, fish).
std::string generate_completion(const std::string& shell);

} // namespace dcpwizard
