#!/usr/bin/env bash
set -euo pipefail

BUCKET="${POLYRAG_S3_BUCKET:-polyrag-local}"
REGION="${AWS_DEFAULT_REGION:-us-east-1}"

awslocal s3api create-bucket \
  --bucket "${BUCKET}" \
  --region "${REGION}" \
  --create-bucket-configuration "LocationConstraint=${REGION}" \
  2>/dev/null || true

awslocal s3api list-buckets
