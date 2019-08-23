
REGISTORY := registry.gitlab.com/termoshtt/cport

all: debian

login:
	docker login $(REGISTORY)

debian:
	docker build -t $(REGISTORY)/debian -f debian.Dockerfile .

push: login debian
	docker push $(REGISTORY)/debian
