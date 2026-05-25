#include <CLI/CLI.hpp>
#include <spdlog/spdlog.h>

#include <filesystem>
#include <string>
#include <thread>

#include "dcpwizard/dcpwizard.h"
#include "dcpwizard/encode.h"
#include "dcpwizard/encrypt.h"
#include "dcpwizard/subtitle.h"
#include "dcpwizard/audio.h"
#include "dcpwizard/colour.h"
#include "dcpwizard/kdm.h"
#include "dcpwizard/reel.h"
#include "dcpwizard/transcode.h"
#include "dcpwizard/atmos.h"
#include "dcpwizard/stereo3d.h"
#include "dcpwizard/verify.h"
#include "dcpwizard/copy_drive.h"
#include "dcpwizard/vf.h"
#include "dcpwizard/loudness.h"
#include "dcpwizard/job_queue.h"
#include "dcpwizard/rest_api.h"
#include "dcpwizard/burnin.h"
#include "dcpwizard/report.h"
#include "dcpwizard/info.h"
#include "dcpwizard/profiles.h"
#include "dcpwizard/watch.h"
#include "dcpwizard/shell_completion.h"

namespace fs = std::filesystem;

int main(int argc, char** argv)
{
  CLI::App app{"DCP Wizard — Digital Cinema Package creator"};
  app.require_subcommand(1);

  // Global options
  bool verbose = false;
  app.add_flag("-v,--verbose", verbose, "Enable verbose output");

  // --- create ---
  auto* create_cmd = app.add_subcommand("create", "Create a new DCP");
  std::string title;
  std::string video_dir;
  std::string audio_file;
  std::string output_dir;
  std::string standard_str = "smpte";
  std::string profile_name;
  bool encrypt = false;

  create_cmd->add_option("--title,-t", title, "DCP title")->required();
  create_cmd->add_option("--video", video_dir, "Video/image sequence directory")->required();
  create_cmd->add_option("--audio", audio_file, "Audio WAV file");
  create_cmd->add_option("--output,-o", output_dir, "Output directory")->required();
  create_cmd->add_option("--standard", standard_str, "DCP standard (smpte|interop)");
  create_cmd->add_option("--profile", profile_name, "Delivery profile");
  create_cmd->add_flag("--encrypt", encrypt, "Encrypt the DCP");

  // --- encode ---
  auto* encode_cmd = app.add_subcommand("encode", "Encode images to JPEG 2000");
  std::string encode_input;
  std::string encode_output;
  uint32_t bandwidth = 250;

  encode_cmd->add_option("--input,-i", encode_input, "Input image directory")->required();
  encode_cmd->add_option("--output,-o", encode_output, "Output J2K directory")->required();
  encode_cmd->add_option("--bandwidth", bandwidth, "Target bitrate (Mbps)");

  // --- transcode ---
  auto* transcode_cmd = app.add_subcommand("transcode", "Transcode video to image sequence");
  std::string transcode_input;
  std::string transcode_output;

  transcode_cmd->add_option("--input,-i", transcode_input, "Input video file")->required();
  transcode_cmd->add_option("--output,-o", transcode_output, "Output directory")->required();

  // --- verify ---
  auto* verify_cmd = app.add_subcommand("verify", "Verify an existing DCP");
  std::string verify_dir;
  verify_cmd->add_option("dcp_dir", verify_dir, "DCP directory")->required();

  // --- info ---
  auto* info_cmd = app.add_subcommand("info", "Show DCP metadata");
  std::string info_dir;
  info_cmd->add_option("dcp_dir", info_dir, "DCP directory")->required();

  // --- kdm ---
  auto* kdm_cmd = app.add_subcommand("kdm", "Generate KDM for encrypted DCP");
  std::string kdm_dcp_dir;
  std::string kdm_cert;
  std::string kdm_output;

  kdm_cmd->add_option("--dcp", kdm_dcp_dir, "Encrypted DCP directory")->required();
  kdm_cmd->add_option("--cert", kdm_cert, "Recipient certificate")->required();
  kdm_cmd->add_option("--output,-o", kdm_output, "Output KDM file")->required();

  // --- copy ---
  auto* copy_cmd = app.add_subcommand("copy", "Copy DCP to drive");
  std::string copy_src;
  std::string copy_dst;

  copy_cmd->add_option("--src", copy_src, "DCP directory")->required();
  copy_cmd->add_option("--dst", copy_dst, "Destination drive/directory")->required();

  // --- loudness ---
  auto* loudness_cmd = app.add_subcommand("loudness", "Measure audio loudness");
  std::string loudness_file;
  loudness_cmd->add_option("audio_file", loudness_file, "Audio file")->required();

  // --- report ---
  auto* report_cmd = app.add_subcommand("report", "Generate QC report");
  std::string report_dcp_dir;
  std::string report_output;

  report_cmd->add_option("--dcp", report_dcp_dir, "DCP directory")->required();
  report_cmd->add_option("--output,-o", report_output, "Output HTML file")->required();

  // --- serve ---
  auto* serve_cmd = app.add_subcommand("serve", "Start REST API server");
  uint16_t port = 8080;
  serve_cmd->add_option("--port,-p", port, "Listen port");

  // --- watch ---
  auto* watch_cmd = app.add_subcommand("watch", "Watch directory for auto-DCP creation");
  std::string watch_dir;
  watch_cmd->add_option("dir", watch_dir, "Directory to watch")->required();

  // --- completion ---
  auto* completion_cmd = app.add_subcommand("completion", "Generate shell completion");
  std::string shell_name = "bash";
  completion_cmd->add_option("shell", shell_name, "Shell (bash|zsh|fish)");

  // --- daemon ---
  auto* daemon_cmd = app.add_subcommand("daemon", "Start job queue daemon");
  std::string daemon_socket = "/tmp/dcpwizard.sock";
  daemon_cmd->add_option("--socket,-s", daemon_socket, "Unix socket path");

  // --- batch ---
  auto* batch_cmd = app.add_subcommand("batch", "Manage job queue");
  auto* batch_list_cmd = batch_cmd->add_subcommand("list", "List all jobs");
  auto* batch_add_cmd  = batch_cmd->add_subcommand("add", "Submit a new job");
  auto* batch_cancel_cmd = batch_cmd->add_subcommand("cancel", "Cancel a job");
  batch_cmd->require_subcommand(1);

  std::string batch_type_str = "create";
  std::string batch_desc;
  std::vector<std::string> batch_args;
  uint64_t batch_cancel_id = 0;

  batch_add_cmd->add_option("-T,--type", batch_type_str, "Job type")->required();
  batch_add_cmd->add_option("-d,--desc", batch_desc, "Job description")->required();
  batch_add_cmd->add_option("args", batch_args, "Job arguments");

  batch_cancel_cmd->add_option("id", batch_cancel_id, "Job ID to cancel")->required();

  CLI11_PARSE(app, argc, argv);

  if (verbose)
    spdlog::set_level(spdlog::level::debug);

  if (create_cmd->parsed())
  {
    dcpwizard::DCPConfig config;
    config.title = title;
    config.standard = (standard_str == "interop") ? dcpwizard::Standard::Interop
                                                  : dcpwizard::Standard::SMPTE;
    config.encrypt = encrypt;
    config.video_dir = video_dir;
    config.audio_file = audio_file;
    config.output_dir = output_dir;
    if (!profile_name.empty())
    {
      // TODO: apply profile settings
    }
    return dcpwizard::create_dcp(config);
  }

  if (encode_cmd->parsed())
  {
    dcpwizard::EncodeConfig config;
    config.input_dir = encode_input;
    config.output_dir = encode_output;
    config.bandwidth_mbps = bandwidth;
    return dcpwizard::encode_j2k(config);
  }

  if (transcode_cmd->parsed())
  {
    dcpwizard::TranscodeConfig config;
    config.input_file = transcode_input;
    config.output_dir = transcode_output;
    return dcpwizard::transcode_to_sequence(config);
  }

  if (verify_cmd->parsed())
  {
    auto result = dcpwizard::verify_dcp(verify_dir);
    if (result.passed)
    {
      spdlog::info("DCP verification PASSED");
      return 0;
    }
    for (const auto& e : result.errors)
      spdlog::error("{}", e);
    for (const auto& w : result.warnings)
      spdlog::warn("{}", w);
    return 1;
  }

  if (info_cmd->parsed())
  {
    auto info = dcpwizard::inspect_dcp(info_dir);
    spdlog::info("Title: {}", info.title);
    spdlog::info("Standard: {}", info.standard);
    spdlog::info("Resolution: {}", info.resolution);
    spdlog::info("Frame rate: {}", info.frame_rate);
    spdlog::info("Duration: {} frames", info.duration_frames);
    spdlog::info("Reels: {}", info.reel_count);
    spdlog::info("Encrypted: {}", info.encrypted ? "yes" : "no");
    return 0;
  }

  if (kdm_cmd->parsed())
  {
    dcpwizard::KDMConfig config;
    config.dcp_dir = kdm_dcp_dir;
    config.certificate = kdm_cert;
    config.output_file = kdm_output;
    return dcpwizard::generate_kdm(config);
  }

  if (copy_cmd->parsed())
    return dcpwizard::copy_to_drive(copy_src, copy_dst);

  if (loudness_cmd->parsed())
  {
    auto result = dcpwizard::measure_loudness(loudness_file);
    spdlog::info("Integrated: {:.1f} LUFS", result.integrated_lufs);
    spdlog::info("True Peak: {:.1f} dBTP", result.true_peak_dbtp);
    spdlog::info("LRA: {:.1f} LU", result.lra_lu);
    return result.passed ? 0 : 1;
  }

  if (report_cmd->parsed())
    return dcpwizard::generate_report(report_dcp_dir, report_output);

  if (serve_cmd->parsed())
    return dcpwizard::start_rest_api(port);

  if (watch_cmd->parsed())
    return dcpwizard::watch_directory(watch_dir, [](const fs::path& p) {
      spdlog::info("New content detected: {}", p.string());
    });

  if (completion_cmd->parsed())
  {
    fmt::print("{}", dcpwizard::generate_completion(shell_name));
    return 0;
  }

  if (daemon_cmd->parsed())
  {
    spdlog::info("Starting job queue daemon (socket: {})", daemon_socket);
    dcpwizard::start_job_queue(daemon_socket);
    // Keep running until terminated
    while (true)
      std::this_thread::sleep_for(std::chrono::seconds(1));
    dcpwizard::stop_job_queue();
    return 0;
  }

  if (batch_cmd->parsed())
  {
    if (batch_list_cmd->parsed())
    {
      auto jobs = dcpwizard::list_jobs();
      if (jobs.empty())
      {
        fmt::print("No jobs in queue\n");
        return 0;
      }
      fmt::print("{:<6} {:<12} {:<10} {:<10} {}\n", "ID", "State", "Progress", "Type", "Description");
      fmt::print("{:-<6} {:-<12} {:-<10} {:-<10} {:-<20}\n", "", "", "", "", "");
      for (const auto& j : jobs)
      {
        fmt::print("{:<6} {:<12} {:<10.0f}% {:<10} {}\n",
          j.id,
          dcpwizard::job_state_to_string(j.state),
          j.progress * 100.0f,
          dcpwizard::job_type_to_string(j.type),
          j.description);
      }
      return 0;
    }
    if (batch_add_cmd->parsed())
    {
      dcpwizard::Job job;
      job.type = dcpwizard::job_type_from_string(batch_type_str);
      job.description = batch_desc;
      job.args = batch_args;
      auto id = dcpwizard::submit_job(job);
      fmt::print("Submitted job {}\n", id);
      return 0;
    }
    if (batch_cancel_cmd->parsed())
    {
      if (dcpwizard::cancel_job(batch_cancel_id))
      {
        fmt::print("Cancelled job {}\n", batch_cancel_id);
        return 0;
      }
      else
      {
        fmt::print("Could not cancel job {}\n", batch_cancel_id);
        return 1;
      }
    }
  }

  return 0;
}
