#version 150

uniform sampler2D albedo;

in vec2 f_uv;
in vec4 f_color;

out vec4 color;

void main() {
	color = texture(albedo, f_uv.st);
	color.r = pow(color.r, 2.2);
	color.g = pow(color.g, 2.2);
	color.b = pow(color.b, 2.2);
	color.a = 1.0 - pow(1.0 - color.a, 2.2);
}
