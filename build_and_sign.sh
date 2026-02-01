#!/bin/bash

# --- Variabel ---
IMAGE_NAME="itrndyovision/rai-openai-based-simulator"  # Ganti dengan username/nama_repo Anda
TAG="pusintelad"  # Atau tag lain yang sesuai
VCS_URL="https://github.com/your_username/your_repo"  # Ganti dengan URL repositori Anda yang BENAR
VCS_REF=$(git rev-parse HEAD)

# --- Build Image (dengan buildx dan provenance/SBOM) ---
docker buildx build \
    --provenance=true \
    --sbom=true \
    --build-arg VCS_URL="$VCS_URL" \
    --build-arg VCS_REF="$VCS_REF" \
    -t "$IMAGE_NAME:$TAG" .  # Hapus --push

# --- Sign Image ---
cosign sign --key cosign.key "$IMAGE_NAME:$TAG"

# --- Verify (Opsional) ---
cosign verify --key cosign.pub "$IMAGE_NAME:$TAG"