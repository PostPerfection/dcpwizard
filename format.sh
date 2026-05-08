find . \( -type d \( -name extern -o -name build \) -prune \) \
-o \( -iname *.h -o -iname *.c -o -iname *.cpp -o -iname *.hpp \) -print \
| xargs clang-format -style=file -i -fallback-style=none
