#version 150 core

uniform sampler2D tex;

in vec2 f_uv;
in vec4 f_color;

out vec4 Target0;

void main() {
	Target0 = f_color * texture(tex, f_uv.st);
	Target0.r = pow(Target0.r, 2.2);
	Target0.g = pow(Target0.g, 2.2);
	Target0.b = pow(Target0.b, 2.2);
	Target0.a = 1.0 - pow(1.0 - Target0.a, 2.2);
}
