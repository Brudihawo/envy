#!/bin/bash

if [ ! -d ./assets ]; then
  mkdir assets
fi
rm -rfv ./assets/*

pushd assets
git clone https://github.com/mathjax/MathJax.git mathjax --depth=1
wget https://raw.githubusercontent.com/sindresorhus/github-markdown-css/main/github-markdown-dark.css
popd
