#version 150 core

layout (std140) uniform VertexArgs {
	uniform vec4 proj_vec;
	uniform vec2 dimension;
};

in vec3 position;
in vec2 tex_coord;
/* in vec4 col;*/

out vec2 f_uv;

void main() {
	vec4 position = vec4(position, 1);
	position *= vec4(dimension, 1, 1);
	position *= proj_vec;
	position += vec4(-1, 1, 0, 0);

	f_uv = tex_coord;
	gl_Position = position;
}
