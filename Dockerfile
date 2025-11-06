FROM rust:latest as builder 

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
FROM rust:latest

WORKDIR /app

RUN apt-get update && apt-get install -y python3 bash && rm -rf /var/lib/apt/lists/*

COPY . .

# Create config directory and copy config file to the expected location
RUN mkdir -p config && \
    cargo build --release && \
    chmod +x ./scripts/*.sh && \
    mkdir -p backend1 backend2 backend3 && \
    echo "<h1>Backend 1 - Server 8081</h1>" > backend1/index.html && \
    echo "<h1>Backend 2 - Server 8082</h1>" > backend2/index.html && \
    echo "<h1>Backend 3 - Server 8083</h1>" > backend3/index.html

EXPOSE 3000 8081 8082 8083

CMD ["bash", "-c", "python3 -m server 8081 --directory /app/backend1 & python3 -m server 8082 --directory /app/backend2 & python3 -m server 8083 --directory /app/backend3 & ./target/release/load-balancer-rs"]
