# Key generation tool
add_executable(keygen tools/keygen.c)
target_link_libraries(keygen
    ${OPENSSL_LIBRARIES}
)
