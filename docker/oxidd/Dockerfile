FROM ubuntu:24.04

RUN apt-get update -qq && apt-get install -y -qq \
    build-essential \
    git \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Rust toolchain (local witness image only — never built or run in CI, ADR-0102).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /opt/oxidd

# Pin by commit SHA (oxidd/oxidd main @ c7a458df8b747a3ee44d0e9e61059a1621865fcd).
RUN git clone --depth 1 https://github.com/oxidd/oxidd.git . \
    && git fetch --depth 1 origin c7a458df8b747a3ee44d0e9e61059a1621865fcd \
    && git checkout c7a458df8b747a3ee44d0e9e61059a1621865fcd

# Counting binary for the local differential (outside CI).
# Reads on stdin: n (number of variables) then 2^n values 0/1 (truth table).
# Writes on stdout: the OxiDD BDD node count (one integer, then a newline).
COPY oxidd_count/ /opt/oxidd_count/
WORKDIR /opt/oxidd_count
RUN cargo build --release \
    && cp target/release/oxidd_count /usr/local/bin/oxidd_count \
    && printf '2\n0 1 1 0\n' | oxidd_count

CMD ["/bin/bash"]
