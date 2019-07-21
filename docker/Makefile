
REGISTORY := registry.gitlab.com/termoshtt/cmake-container-builder

all: debian

login:
	docker login $(REGISTORY)

debian:
	docker build -t $(REGISTORY)/debian -f debian.Dockerfile .

push: login debian
	docker push $(REGISTORY)/debian
