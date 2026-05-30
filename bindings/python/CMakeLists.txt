# Python bindings for DCP Wizard
# These are subprocess-based wrappers around the CLI binary.
# No compilation needed — just install the Python package.
#
# Usage:
#   pip install -e bindings/python/
#   or: DCPWIZARD_BIN=/path/to/dcpwizard python -c "import dcpwizard; ..."

set_target_properties(pydcpwizard PROPERTIES
  OUTPUT_NAME "_dcpwizard"
  PREFIX ""
)

if(Python3_SOABI)
  set_target_properties(pydcpwizard PROPERTIES SUFFIX ".${Python3_SOABI}.so")
endif()

install(TARGETS pydcpwizard DESTINATION ${Python3_SITEARCH})
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/dcpwizard.py DESTINATION ${Python3_SITEARCH})
