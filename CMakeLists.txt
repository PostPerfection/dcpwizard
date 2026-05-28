cmake_minimum_required(VERSION 3.25)
project(dcpwizard
    VERSION 1.0.0
    DESCRIPTION "DCP Wizard — Digital Cinema Package creator"
    LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find cargo
find_program(CARGO_EXECUTABLE cargo REQUIRED)

# Determine build profile
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(CARGO_BUILD_FLAGS --release)
    set(CARGO_TARGET_DIR "${CMAKE_SOURCE_DIR}/rust/target/release")
else()
    set(CARGO_BUILD_FLAGS "")
    set(CARGO_TARGET_DIR "${CMAKE_SOURCE_DIR}/rust/target/debug")
endif()

# Build Rust workspace
add_custom_target(dcpwizard_rust ALL
    COMMAND ${CARGO_EXECUTABLE} build ${CARGO_BUILD_FLAGS}
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/rust
    COMMENT "Building dcpwizard Rust workspace")

# Install CLI binary
install(PROGRAMS ${CARGO_TARGET_DIR}/dcpwizard-cli
    DESTINATION bin
    RENAME dcpwizard
    OPTIONAL)

# Tests
option(BUILD_TESTING "Build tests" ON)
if(BUILD_TESTING)
    enable_testing()

    # Rust unit tests
    add_test(NAME rust_tests
        COMMAND ${CARGO_EXECUTABLE} test
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/rust)

    # Rust clippy lint
    add_test(NAME rust_clippy
        COMMAND ${CARGO_EXECUTABLE} clippy --all-targets --all-features -- -D warnings
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/rust)

    # CLI integration tests (Python)
    find_package(Python3 COMPONENTS Interpreter)
    if(Python3_FOUND)
        add_test(NAME dcpwizard_cli_test
            COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/tests/test_cli.py
            WORKING_DIRECTORY ${CMAKE_BINARY_DIR})

        # GUI launch/exit tests (Python, optional)
        add_test(NAME dcpwizard_gui_test
            COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/tests/test_gui.py)
    endif()
endif()

# Python bindings (SWIG)
option(BUILD_PYTHON_BINDINGS "Build Python bindings via SWIG" OFF)
if(BUILD_PYTHON_BINDINGS)
  add_subdirectory(bindings/python)
endif()
