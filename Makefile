# This Makefile is meant for shader compilation only.
# Use cargo to compile the rust part of the project.

GLSLC = $(shell ./find_glslc.sh)
ifeq "$(GLSLC)" ""
	break;
endif

FLAGS = -c -g

SHADERS=$(wildcard src/shaders/*)
COMP_SHADERS = $(patsubst src/shaders/%,compiled/%.spv,$(SHADERS))
COMP_DISASMS = $(patsubst src/shaders/%,compiled/%.spvasm,$(SHADERS))


all: $(COMP_SHADERS) $(COMP_DISASMS)

compiled/%.spv: src/shaders/%
	mkdir -p $(dir $@)
	$(GLSLC) -MD -c -O -o $@ $<

compiled/%.spvasm: src/shaders/%
	mkdir -p $(dir $@)
	$(GLSLC) -MD -S -g -O -o $@ $<

clean:
	rm compiled/*.spv compiled/*.spvasm compiled/*.d

.PHONY: all clean