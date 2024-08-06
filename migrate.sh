#!/bin/bash

for file in $(fd .md); do
  # sed -i 's/\\(/$/g' $file
  # sed -i 's/\\)/$/g' $file
  # sed -i 's/\\\[/$$/g' $file
  # sed -i 's/\\\]/$$/g' $file
  echo $file
  grep '${0,1}\\left{' $file
done
