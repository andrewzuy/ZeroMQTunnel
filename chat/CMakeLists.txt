cmake_minimum_required(VERSION 3.10)
cmake_minimum_required(VERSION 3.10)
project(Chat VERSION 1.0.0 LANGUAGES C)

find_package(ZeroMQ REQUIRED)
find_package(OpenSSL REQUIRED)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

add_executable(chat-server
    src/server.c
    src/protocol.c
    src/crypto.c
    src/utils.c
    src/logging.c
)

target_include_directories(chat-server PRIVATE
    ${CMAKE_SOURCE_DIR}/include
    ${ZeroMQ_INCLUDE_DIRS}
)

target_link_libraries(chat-server
    ${ZerOmq_LIBRARIES}
    ${OPENSSL_LIBRARIES}
)

add_executable(chat-client
    src/client.c
    src/protocol.c
    src/crypto.c
    src/utils.c
    src/logging.c
)

target_include_directories(chat-client PRIVATE
    ${CMAKE_SOURCE_DIR}/include
    ${ZeroMQ_INCLUDE_DIRS}
)

target_link_libraries(chat-client
    ${ZerOmq_LIBRARIES}
    ${OPENSSL_LIBRARIES}
)