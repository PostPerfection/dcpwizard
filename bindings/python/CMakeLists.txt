cmake_minimum_required(VERSION 3.25)

find_package(SWIG 4.0 REQUIRED)
find_package(Python3 REQUIRED COMPONENTS Interpreter Development)
include(UseSWIG)

set_property(SOURCE dcpwizard.i PROPERTY CPLUSPLUS ON)
set_property(SOURCE dcpwizard.i PROPERTY USE_TARGET_INCLUDE_DIRECTORIES ON)

swig_add_library(pydcpwizard
  TYPE SHARED
  LANGUAGE python
  SOURCES dcpwizard.i
)

target_include_directories(pydcpwizard PRIVATE
  ${PROJECT_SOURCE_DIR}/include
  ${Python3_INCLUDE_DIRS}
)

target_link_libraries(pydcpwizard PRIVATE
  dcpwizard_core
  Python3::Python
)

set_target_properties(pydcpwizard PROPERTIES
  OUTPUT_NAME "_dcpwizard"
  PREFIX ""
)

if(Python3_SOABI)
  set_target_properties(pydcpwizard PROPERTIES SUFFIX ".${Python3_SOABI}.so")
endif()

install(TARGETS pydcpwizard DESTINATION ${Python3_SITEARCH})
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/dcpwizard.py DESTINATION ${Python3_SITEARCH})
