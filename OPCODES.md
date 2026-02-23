# Toka Instruction Set (v0.0)

**Primary numeric type:** S44 (Spirix Scalar<i16, i16> - 32-bit, 16-bit fraction + 16-bit exponent)

## Stack Manipulation
```
push <value>     - Push VSF-encoded constant to stack
push_zero        - Push 0.0 (optimized)
push_one         - Push 1.0 (optimized)
pop              - Discard top of stack
dup              - Duplicate top value
dup_n <depth>    - Duplicate value at depth N
swap             - Swap top two values
rotate           - Rotate top three (a b c → b c a)
```

## Local Variables
```
local_alloc <n>  - Allocate N local variable slots
local_get <id>   - Push local[id] to stack
local_set <id>   - Pop stack to local[id]
local_tee <id>   - Copy top to local[id] without popping
```

## Arithmetic (Spirix S44)
```
add              - Pop b, a; push a + b
sub              - Pop b, a; push a - b
mul              - Pop b, a; push a * b
div              - Pop b, a; push a / b
recip            - Pop a; push 1/a (faster than div)
mod              - Pop b, a; push a % b
neg              - Pop a; push -a
abs              - Pop a; push |a|
sqrt             - Pop a; push √a
pow              - Pop exp, base; push base^exp
min              - Pop b, a; push min(a, b)
max              - Pop b, a; push max(a, b)
clamp            - Pop max, min, value; push clamped value
floor            - Pop a; push ⌊a⌋
ceil             - Pop a; push ⌈a⌉
round            - Pop a; push round(a)
frac             - Pop a; push fractional part
lerp             - Pop t, b, a; push a + t*(b-a)
```

## Trigonometry
```
sin              - Pop a; push sin(a)
cos              - Pop a; push cos(a)
tan              - Pop a; push tan(a)
asin             - Pop a; push arcsin(a)
acos             - Pop a; push arccos(a)
atan             - Pop a; push arctan(a)
atan2            - Pop y, x; push atan2(y, x)
```

## Comparison (returns 1.0 or 0.0 as S44)
```
eq               - Pop b, a; push 1.0 if a == b else 0.0
ne               - Pop b, a; push 1.0 if a != b else 0.0
lt               - Pop b, a; push 1.0 if a < b else 0.0
le               - Pop b, a; push 1.0 if a ≤ b else 0.0
gt               - Pop b, a; push 1.0 if a > b else 0.0
ge               - Pop b, a; push 1.0 if a ≥ b else 0.0
```

## Logic (0.0 = false, non-zero = true)
```
and              - Pop b, a; push 1.0 if both non-zero else 0.0
or               - Pop b, a; push 1.0 if either non-zero else 0.0
not              - Pop a; push 1.0 if zero else 0.0
```

## Type System (VSF types on stack)
```
typeof           - Pop value; push type identifier as string (d-type)
to_s44           - Pop value; convert to s44, push result
to_u32           - Pop value; convert to u5 (u32), push result
to_string        - Pop value; convert to x (UTF-8 string), push result
```

## Arrays
```
array_new        - Pop count; create array with count elements from stack
array_len        - Pop array; push length (S44)
array_get        - Pop index, array; push element
array_set        - Pop value, index, array; modify in place
array_push       - Pop value, array; append element
array_pop        - Pop array; push & remove last element
```

## Strings (UTF-8)
```
string_concat    - Pop b, a; push a + b
string_len       - Pop string; push byte length (S44)
string_slice     - Pop end, start, string; push substring
```

## Handles (capability-bounded references)
```
handle_read      - Pop handle; push referenced value
handle_write     - Pop value, handle; write to handle (if writable)
handle_call      - Pop args..., handle; invoke capability function
handle_query     - Pop handle; push metadata struct
```

## Drawing (viewport-relative 0.0-1.0 as S44)
```
clear            - Pop r, g, b, a (S44); clear canvas to colour
fill_rect        - Pop r, g, b, a (S44), h, w, y, x; fill rectangle
stroke_rect      - Pop r, g, b, a (S44), stroke_w, h, w, y, x; stroke outline
fill_circle      - Pop r, g, b, a (S44), r, cy, cx; fill circle
stroke_circle    - Pop r, g, b, a (S44), stroke_w, r, cy, cx; stroke outline
draw_line        - Pop r, g, b, a (S44), stroke_w, y2, x2, y1, x1; draw line
draw_text        - Pop r, g, b, a (S44), size, y, x, string; render text
set_font         - Pop font_handle; set current font

Note: Colour constants (rck, rcr, rcb, etc.) are expanded to 4 S44 RGBA components by the push opcode.
```

## Color Utilities
```
rgba             - Pop a, b, g, r (S44 0.0-1.0); push u32 RGBA
rgb              - Pop b, g, r (S44 0.0-1.0); push u32 RGBA (alpha=1.0)
colour_lerp      - Pop t, colour_b, colour_a; push interpolated u32 RGBA
hsla             - Pop a, l, s, h; push u32 RGBA
```

## Control Flow
```
call <offset>    - Call function at bytecode offset
call_indirect    - Pop function_handle; call it
return           - Return from function (no value)
return_value     - Pop value; return it from function
jump <offset>    - Unconditional jump to offset
jump_if <offset> - Pop condition; jump if non-zero
jump_zero <off>  - Pop condition; jump if zero
```

## Random Numbers
```
random           - Push random S44 in [-1.0, 1.0]
random_gauss     - Push random S44 (Gaussian distribution)
random_range     - Pop max, min; push random S44 in [min, max]
```

## Cryptography (capability-bounded)
```
blake3           - Pop data; push 32-byte BLAKE3 hash
```

## Time
```
timestamp        - Push current Unix timestamp (S44 seconds)
```

## Error Handling
```
assert           - Pop condition; halt if zero
halt             - Stop execution immediately
```

## Debug (only available with debug capability)
```
debug_print      - Pop value; print to debug console
debug_stack      - Print entire stack state
nop              - No operation
```

## Future Extensions (not v0)
```
# File I/O (capability-bounded)
file_read        - Pop path_handle; push file contents
file_write       - Pop data, path_handle; write to file

# Network (capability-bounded)
net_fetch        - Pop url; push response data

# Module System
import           - Pop capsule_hash; load and link module

# Concurrency
spawn            - Pop capsule_hash; spawn parallel execution
```