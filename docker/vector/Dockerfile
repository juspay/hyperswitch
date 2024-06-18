FROM debian:bookworm-slim as builder
RUN apt update
RUN apt install -y lua5.4 liblua5.4-dev wget git make unzip openssl libssl-dev cmake
RUN wget https://luarocks.org/releases/luarocks-3.11.0.tar.gz
RUN tar -zxf luarocks-3.11.0.tar.gz
RUN cd luarocks-3.11.0 && ./configure && make && make install
RUN git clone https://github.com/rxi/json.lua.git --depth 1 && cp json.lua/json.lua /usr/local/share/lua/5.4
RUN git clone https://github.com/lsampras/lua-cassandra.git --depth 1
RUN cd lua-cassandra && luarocks make lua-cassandra-dev-0.rockspec

FROM docker.io/timberio/vector:latest-debian
LABEL modified_by=lsampras@pm.me DOCKERFILE_SRC=https://gist.github.com/lsampras/83d515351bd5ff106e1b7cf477d3e7d3 IMAGE_SRC=docker.io/lsampras/vector
COPY --from=builder /usr/local/share/lua/5.4 /usr/local/share/lua/5.4
COPY --from=builder /usr/local/lib/lua/5.4 /usr/local/lib/lua/5.4
