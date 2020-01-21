# Dockerfile that builds an image with the Substrate example node using the
# names pallet and its frontend.  During the build process also the unit
# tests are run.

FROM parity/rust-builder AS builder
RUN apt update && apt install -y yarnpkg

# Copy over all sources for the names pallet.
WORKDIR /usr/src/substrate-names
COPY . .

# Build production frontend.
WORKDIR /usr/src/substrate-names/frontend
RUN yarnpkg install && yarnpkg build

# Run unit tests.
WORKDIR /usr/src/substrate-names/names
RUN cargo test --lib

# Build the node binary.
WORKDIR /usr/src/substrate-names/node
RUN cargo build --release

# Assemble the final image just with binary and production frontend.
FROM debian:buster
RUN apt update && apt install -y nginx
COPY --from=builder \
  /usr/src/substrate-names/node/target/release/node /usr/local/bin/
COPY --from=builder /usr/src/substrate-names/frontend/build /var/www/html/
COPY docker/entrypoint.sh /usr/local/bin/

# Provide meta data.
LABEL description="The example Substrate node using names and its frontend"
EXPOSE 80
EXPOSE 9944
VOLUME ["/var/lib/node"]
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["node", "--dev"]
