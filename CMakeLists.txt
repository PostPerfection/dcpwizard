cmake_minimum_required(VERSION 3.25)
project(dcpwizard
    VERSION 0.1.0
    DESCRIPTION "DCP Wizard — Digital Cinema Package creator"
    LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Python bindings require position-independent code for all static libs
option(BUILD_PYTHON_BINDINGS "Build Python bindings via SWIG" OFF)
if(BUILD_PYTHON_BINDINGS)
  set(CMAKE_POSITION_INDEPENDENT_CODE ON)
endif()

# Dependencies
if(WIN32)
    find_package(LibXml2 REQUIRED)
    find_package(OpenSSL REQUIRED)
    find_package(XercesC REQUIRED)
else()
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(LIBXML2 REQUIRED IMPORTED_TARGET libxml-2.0)
    pkg_check_modules(OPENSSL REQUIRED IMPORTED_TARGET openssl)
    pkg_check_modules(XERCES REQUIRED IMPORTED_TARGET xerces-c)
endif()

# CLI11 (header-only)
add_subdirectory(extern/CLI11 EXCLUDE_FROM_ALL)

# spdlog
add_subdirectory(extern/spdlog EXCLUDE_FROM_ALL)

# asdcplib
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
add_subdirectory(extern/asdcplib EXCLUDE_FROM_ALL)

# dcpdoctor (validation/QC library)
add_subdirectory(extern/dcpdoctor EXCLUDE_FROM_ALL)

# Core library
add_library(dcpwizard_core STATIC
    src/dcp.cpp
    src/encode.cpp
    src/encrypt.cpp
    src/subtitle.cpp
    src/audio.cpp
    src/colour.cpp
    src/kdm.cpp
    src/reel.cpp
    src/cpl.cpp
    src/pkl.cpp
    src/assetmap.cpp
    src/hash.cpp
    src/mxf_wrap.cpp
    src/transcode.cpp
    src/atmos.cpp
    src/stereo3d.cpp
    src/markers.cpp
    src/verify.cpp
    src/copy_drive.cpp
    src/vf.cpp
    src/loudness.cpp
    src/job_queue.cpp
    src/rest_api.cpp
    src/burnin.cpp
    src/report.cpp
    src/info.cpp
    src/profiles.cpp
    src/preferences.cpp
    src/watch.cpp
    src/shell_completion.cpp
    src/geometry.cpp
    src/import.cpp
    src/j2k_transcode.cpp
    src/multi_cpl.cpp
    src/dtsx.cpp
    src/hfr.cpp
    src/kdm_advanced.cpp
    src/export.cpp
    src/qc.cpp)

target_include_directories(dcpwizard_core PUBLIC
    ${CMAKE_SOURCE_DIR}/include
    ${CMAKE_SOURCE_DIR}/extern/asdcplib/src)

if(WIN32)
    target_compile_definitions(dcpwizard_core PUBLIC KM_WIN32)
    target_link_libraries(dcpwizard_core PUBLIC
        LibXml2::LibXml2
        OpenSSL::SSL OpenSSL::Crypto
        XercesC::XercesC
        spdlog::spdlog
        libasdcp
        libkumu
        dcpdoctor_core
        ws2_32)
else()
    target_link_libraries(dcpwizard_core PUBLIC
        PkgConfig::LIBXML2
        PkgConfig::OPENSSL
        PkgConfig::XERCES
        spdlog::spdlog
        libasdcp
        libkumu
        dcpdoctor_core)
endif()

# CLI tool
add_executable(dcpwizard src/main.cpp)
target_link_libraries(dcpwizard PRIVATE dcpwizard_core CLI11::CLI11)

# Tests
option(BUILD_TESTING "Build tests" ON)
if(BUILD_TESTING)
    enable_testing()
    add_executable(dcpwizard_test tests/test_main.cpp)
    target_link_libraries(dcpwizard_test PRIVATE dcpwizard_core)
    add_test(NAME dcpwizard_test COMMAND dcpwizard_test)

    add_executable(dcpwizard_integration_test tests/test_integration.cpp)
    target_link_libraries(dcpwizard_integration_test PRIVATE dcpwizard_core)
    add_test(NAME dcpwizard_integration_test COMMAND dcpwizard_integration_test)

    add_executable(dcpwizard_job_queue_test tests/test_job_queue.cpp)
    target_link_libraries(dcpwizard_job_queue_test PRIVATE dcpwizard_core)
    add_test(NAME dcpwizard_job_queue_test COMMAND dcpwizard_job_queue_test)

    # CLI integration tests (Python)
    find_package(Python3 REQUIRED COMPONENTS Interpreter)
    add_test(NAME dcpwizard_cli_test
        COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/tests/test_cli.py
        WORKING_DIRECTORY ${CMAKE_BINARY_DIR})

    # GUI launch/exit tests (Python, optional)
    add_test(NAME dcpwizard_gui_test
        COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/tests/test_gui.py)

    # job_queue_test and cli_test both use /tmp/dcpwizard.sock — serialize them
    set_tests_properties(dcpwizard_job_queue_test dcpwizard_cli_test
        PROPERTIES RESOURCE_LOCK "dcpwizard_socket")
endif()

# Python bindings (SWIG)
if(BUILD_PYTHON_BINDINGS)
  add_subdirectory(bindings/python)
endif()
