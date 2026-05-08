#pragma once

#ifdef _WIN32
#  ifdef DCPWIZARD_BUILDING_DLL
#    define DCPWIZARD_API __declspec(dllexport)
#  else
#    define DCPWIZARD_API __declspec(dllimport)
#  endif
#else
#  define DCPWIZARD_API
#endif
