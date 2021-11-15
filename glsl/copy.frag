#version 330 core

uniform sampler2D state;
uniform vec2 scale;

void main() {
    gl_FragColor = texture2D(state, gl_FragCoord.xy / scale);
}
