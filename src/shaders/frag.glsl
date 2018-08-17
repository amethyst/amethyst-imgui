#version 150

uniform sampler2D albedo;

in vec2 f_uv;
in vec4 f_color;

out vec4 color;

void main() {
	color = texture(albedo, f_uv.st);
}
