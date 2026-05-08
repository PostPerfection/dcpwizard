// SWIG interface file for DCP Wizard Python bindings
%module dcpwizard

%{
#include "dcpwizard/dcp.h"
#include "dcpwizard/encode.h"
#include "dcpwizard/encrypt.h"
#include "dcpwizard/subtitle.h"
#include "dcpwizard/audio.h"
#include "dcpwizard/colour.h"
#include "dcpwizard/kdm.h"
#include "dcpwizard/kdm_advanced.h"
#include "dcpwizard/reel.h"
#include "dcpwizard/cpl.h"
#include "dcpwizard/pkl.h"
#include "dcpwizard/assetmap.h"
#include "dcpwizard/hash.h"
#include "dcpwizard/mxf_wrap.h"
#include "dcpwizard/transcode.h"
#include "dcpwizard/atmos.h"
#include "dcpwizard/dtsx.h"
#include "dcpwizard/stereo3d.h"
#include "dcpwizard/markers.h"
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
#include "dcpwizard/geometry.h"
#include "dcpwizard/import.h"
#include "dcpwizard/j2k_transcode.h"
#include "dcpwizard/multi_cpl.h"
#include "dcpwizard/hfr.h"
#include "dcpwizard/export.h"
#include "dcpwizard/qc.h"
#include "dcpwizard/portable.h"
%}

// STL support
%include "std_string.i"
%include "std_vector.i"
%include "stdint.i"

// Map std::filesystem::path to/from Python str
%typemap(in) std::filesystem::path {
  if (!PyUnicode_Check($input)) {
    PyErr_SetString(PyExc_TypeError, "Expected a string");
    SWIG_fail;
  }
  $1 = std::filesystem::path(PyUnicode_AsUTF8($input));
}
%typemap(in) const std::filesystem::path& (std::filesystem::path temp) {
  if (!PyUnicode_Check($input)) {
    PyErr_SetString(PyExc_TypeError, "Expected a string");
    SWIG_fail;
  }
  temp = std::filesystem::path(PyUnicode_AsUTF8($input));
  $1 = &temp;
}
%typemap(out) std::filesystem::path {
  $result = PyUnicode_FromString($1.string().c_str());
}
%typemap(typecheck, precedence=SWIG_TYPECHECK_STRING) std::filesystem::path, const std::filesystem::path& {
  $1 = PyUnicode_Check($input) ? 1 : 0;
}

// Template instantiations
%template(StringVector) std::vector<std::string>;

// Parse the headers
%include "dcpwizard/dcp.h"
%include "dcpwizard/encode.h"
%include "dcpwizard/encrypt.h"
%include "dcpwizard/subtitle.h"
%include "dcpwizard/audio.h"
%include "dcpwizard/colour.h"
%include "dcpwizard/kdm.h"
%include "dcpwizard/kdm_advanced.h"
%include "dcpwizard/reel.h"
%include "dcpwizard/cpl.h"
%include "dcpwizard/pkl.h"
%include "dcpwizard/assetmap.h"
%include "dcpwizard/hash.h"
%include "dcpwizard/mxf_wrap.h"
%include "dcpwizard/transcode.h"
%include "dcpwizard/atmos.h"
%include "dcpwizard/dtsx.h"
%include "dcpwizard/stereo3d.h"
%include "dcpwizard/markers.h"
%include "dcpwizard/verify.h"
%include "dcpwizard/copy_drive.h"
%include "dcpwizard/vf.h"
%include "dcpwizard/loudness.h"
%include "dcpwizard/job_queue.h"
%include "dcpwizard/rest_api.h"
%include "dcpwizard/burnin.h"
%include "dcpwizard/report.h"
%include "dcpwizard/info.h"
%include "dcpwizard/profiles.h"
%include "dcpwizard/watch.h"
%include "dcpwizard/geometry.h"
%include "dcpwizard/import.h"
%include "dcpwizard/j2k_transcode.h"
%include "dcpwizard/multi_cpl.h"
%include "dcpwizard/hfr.h"
%include "dcpwizard/export.h"
%include "dcpwizard/qc.h"
%include "dcpwizard/portable.h"
