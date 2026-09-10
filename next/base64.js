'use strict';

const B64CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

export function encode(b) {
    let blocks = Math.trunc(b.length / 3);
    let r = '';
    let x;
    for (let i = 0; i < blocks; ++i) {
        x = (b[3*i] << 16) | (b[3*i+1] << 8) | b[3*i+2];
        r += B64CHARS[(x >> 18) & 0x3f];
        r += B64CHARS[(x >> 12) & 0x3f];
        r += B64CHARS[(x >> 6) & 0x3f];
        r += B64CHARS[x & 0x3f];
    }
    switch (b.length % 3) {
    case 0:
        break;
    case 1:
        x = (b[b.length - 1] << 16);
        r += B64CHARS[(x >> 18) & 0x3f];
        r += B64CHARS[(x >> 12) & 0x3f];
        r += "==";
        break;
    case 2:
        x = (b[b.length - 2] << 16) | (b[b.length - 1] << 8);
        r += B64CHARS[(x >> 18) & 0x3f];
        r += B64CHARS[(x >> 12) & 0x3f];
        r += B64CHARS[(x >> 6) & 0x3f];
        r += "=";
        break;
    }
    return r;
}

export function decode(s) {
    let len = Math.trunc(s.length / 4) * 3;
    if (s[s.length - 1] == '=') {
        --len;
        if (s[s.length - 2] == '=') {
            --len;
        }
    }
    let b = new Uint8Array(len);
    let blocks = Math.trunc(len / 3);
    let x;
    for (let i = 0; i < blocks; ++i) {
        x = (B64CHARS.indexOf(s[4*i]) << 18) | (B64CHARS.indexOf(s[4*i+1]) << 12) | (B64CHARS.indexOf(s[4*i+2]) << 6) | B64CHARS.indexOf(s[4*i+3]);
        b[3*i] = (x >> 16) & 0xff;
        b[3*i + 1] = (x >> 8) & 0xff;
        b[3*i + 2] = x & 0xff;
    }
    switch (len % 3) {
    case 0:
        break;
    case 1:
        x = (B64CHARS.indexOf(s[4*blocks]) << 18) | (B64CHARS.indexOf(s[4*blocks+1]) << 12);
        b[3*blocks] = (x >> 16) & 0xff;
        break;
    case 2:
        x = (B64CHARS.indexOf(s[4*blocks]) << 18) | (B64CHARS.indexOf(s[4*blocks+1]) << 12) | (B64CHARS.indexOf(s[4*blocks+2]) << 6);
        b[3*blocks] = (x >> 16) & 0xff;
        b[3*blocks + 1] = (x >> 8) & 0xff;
        break;
    }
    return b;    
}
