DOCKER_BUILDKIT ?= 1
export DOCKER_BUILDKIT

IMAGE       ?= polyrag:local
ARTIFACT    := artifacts/polyrag

PLATFORMS   ?= linux/amd64,linux/arm64

.PHONY: binary image image-full image-multiarch buildx-setup clean-artifacts

binary:
	docker build -f dockerfile.build --target export --output type=local,dest=./artifacts .
	@test -f $(ARTIFACT) && echo "==> $(ARTIFACT) ready" || (echo "!! binary missing" && exit 1)

image: binary
	docker build -f dockerfile -t $(IMAGE) .

image-full:
	docker build -f dockerfile.build --target runtime -t $(IMAGE) .

buildx-setup:
	docker run --privileged --rm tonistiigi/binfmt --install all
	docker buildx create --name polyrag-multi --driver docker-container --use || docker buildx use polyrag-multi

image-multiarch:
	docker buildx build -f dockerfile.build --target runtime \
		--platform $(PLATFORMS) -t $(IMAGE) --push .

clean-artifacts:
	rm -rf artifacts
