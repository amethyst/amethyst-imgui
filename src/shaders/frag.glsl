#version 150 core

uniform sampler2D tex;

in vec2 f_uv;

out vec4 Target0;

void main() {
	Target0 = f_color * texture(tex, f_uv.st);
}
