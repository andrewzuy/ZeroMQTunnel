add_library(keygen_crypto_lib STATIC
    src/keygen_crypto.c
)

target_link_libraries(keygen_crypto_lib
    ${OPENSSL_LIBRARIES}
    keycrypto_tests
)

include_directories(../.)
