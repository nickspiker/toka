# Toka Instruction Set - Grouped by Function

## Stack Manipulation
```
push <value>     - Push constant to stack
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

## Arithmetic (Spirix)
```
add              - Pop b, a; push a + b
sub              - Pop b, a; push a - b
mul              - Pop b, a; push a * b
div              - Pop b, a; push a / b
mod              - Pop b, a; push a % b
neg              - Pop a; push -a
abs              - Pop a; push |a|
sqrt             - Pop a; push √a
cbrt             - Pop a; push ∛a
pow              - Pop exp, base; push base^exp
powi <n>         - Pop base; push base^n (integer exponent)
min              - Pop b, a; push min(a, b)
max              - Pop b, a; push max(a, b)
floor            - Pop a; push ⌊a⌋
ceil             - Pop a; push ⌈a⌉
round            - Pop a; push round(a)
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
sinh             - Pop a; push sinh(a)
cosh             - Pop a; push cosh(a)
tanh             - Pop a; push tanh(a)
```

## Comparison
```
eq               - Pop b, a; push a == b
ne               - Pop b, a; push a != b
lt               - Pop b, a; push a < b
le               - Pop b, a; push a ≤ b
gt               - Pop b, a; push a > b
ge               - Pop b, a; push a ≥ b
```

## Logic
```
and              - Pop b, a; push a && b
or               - Pop b, a; push a || b
not              - Pop a; push !a
xor              - Pop b, a; push a ^ b
```

## Bitwise (on u64)
```
bit_and          - Pop b, a; push a & b
bit_or           - Pop b, a; push a | b
bit_xor          - Pop b, a; push a ⊕ b
bit_not          - Pop a; push ~a
shl              - Pop shift, a; push a << shift
shr              - Pop shift, a; push a >> shift
```

## Type Operations
```
cast <type>      - Pop value, cast to type, push result
assert_type <t>  - Pop value, verify type, error if wrong, push back
typeof           - Pop value, push type tag
is_spirix        - Pop value; push bool (is Spirix type?)
is_u64           - Pop value; push bool
is_string        - Pop value; push bool
is_color         - Pop value; push bool
is_bool          - Pop value; push bool
is_handle        - Pop value; push bool
pack <def>       - Pop N fields, pack into struct per definition
unpack           - Pop struct, push all fields to stack
```

## Arrays
```
array_new <t> <n> - Pop n values, create typed array
array_len         - Pop array; push length
array_get         - Pop index, array; push element
array_set         - Pop value, index, array; modify array
array_push        - Pop value, array; append element
array_pop         - Pop array; push & remove last element
array_slice       - Pop end, start, array; push slice
array_concat      - Pop b, a; push concatenated array
```

## Strings
```
string_concat    - Pop b, a; push a + b
string_len       - Pop string; push length
string_slice     - Pop end, start, string; push substring
string_char_at   - Pop index, string; push character
string_format    - Pop args..., fmt; push formatted string
string_parse <t> - Pop string; parse to type, push result
string_split     - Pop delimiter, string; push array of parts
string_join      - Pop separator, array; push joined string
```

## Memory & Buffers
```
buffer_alloc <n> - Allocate n-byte buffer, push handle
buffer_free      - Pop handle; deallocate
buffer_read <o>  - Pop handle; push byte at offset o
buffer_write <o> - Pop value, handle; write to offset o
buffer_copy      - Pop len, dst_off, src_off, dst, src
mem_read <h> <o> - Read from handle h at offset o (capability-checked)
mem_write <h><o> - Pop value; write to handle h at offset o
```

## Drawing (viewport-relative 0.0-1.0)
```
fill_rect        - Pop color, h%, w%, y%, x%; fill rectangle
stroke_rect      - Pop width%, color, h%, w%, y%, x%; stroke outline
fill_circle      - Pop color, r%, cy%, cx%; fill circle
stroke_circle    - Pop width%, color, r%, cy%, cx%; stroke outline
fill_ellipse     - Pop color, ry%, rx%, cy%, cx%; fill ellipse
stroke_ellipse   - Pop width%, color, ry%, rx%, cy%, cx%; stroke outline
draw_line        - Pop width%, color, y2%, x2%, y1%, x1%; draw line
draw_text        - Pop size%, y%, x%, string; render text
draw_path        - Pop color, point_array; fill path
stroke_path      - Pop width%, color, point_array; stroke path
clear_canvas     - Pop color; fill entire viewport
set_color        - Pop color; set current draw color
set_transform    - Pop 3x3 matrix; set coordinate transform
```

## Font Operations
```
font_load        - Pop font_data; load font, push handle
font_set         - Pop font_handle; set as current font
font_measure     - Pop size%, string; push (width%, height%)
font_metrics     - Pop font_handle; push (ascent, descent, line_height)
draw_text_styled - Pop style_struct, size%, y%, x%, string
text_wrap        - Pop max_width%, string; push array of wrapped lines
text_align <m>   - Set text alignment mode (left/center/right/justify)
text_baseline <m>- Set baseline (top/middle/bottom/alphabetic)
glyph_path       - Pop char, font; push vector path for glyph
```

## Handle I/O (capability-bounded)
```
read_handle <id> - Read element by handle id, push value
write_handle <id>- Pop value; write to handle id
call_handle <id> - Invoke capability function at handle id
query_handle <id>- Get handle metadata (type, permissions)
```

## Control Flow
```
call <func_id>   - Call function at index, push return address
call_indirect    - Pop function handle; call it
return           - Return from function
return_value     - Pop value; return it from function
branch <label>   - Unconditional jump to instruction index
branch_if <lbl>  - Pop condition; jump if true
branch_unless <l>- Pop condition; jump if false
loop_start <lbl> - Mark loop start point
loop_end         - Jump back to loop_start
break            - Exit current loop
continue         - Jump to loop_start
```

## I/O Operations
```
read_input <src> - Read from input source (keyboard/file/network)
write_output <d> - Pop value; write to output destination
flush            - Flush output buffers
file_open        - Pop mode, path; push file handle
file_close       - Pop file handle; close it
file_read        - Pop length, handle; push bytes
file_write       - Pop bytes, handle; write to file
```

## Cryptography
```
hash             - Pop data; push BLAKE3 hash
sign             - Pop data, private_key; push signature
verify           - Pop signature, data, public_key; push bool
encrypt          - Pop data, key; push encrypted data
decrypt          - Pop encrypted_data, key; push plaintext
```

## Time
```
timestamp        - Push current Unix timestamp
sleep            - Pop milliseconds; pause execution
deadline         - Pop milliseconds; set timeout for operation
```

## Error Handling
```
try              - Begin try block (push error handler address)
catch            - Handle error (pop error, push to stack)
throw            - Pop error value; throw exception
assert           - Pop condition; error if false
```

## Debug & Introspection
```
debug_print      - Pop value; print to debug console
debug_stack      - Print entire stack state
halt             - Stop execution immediately
nop              - No operation (placeholder)
breakpoint       - Trigger debugger breakpoint
```

## Concurrency (future)
```
spawn            - Pop capsule_hash; spawn parallel execution
lock             - Pop handle; acquire lock
unlock           - Pop handle; release lock
atomic_swap      - Pop new, old, handle; atomic compare-and-swap
```

## Module System
```
import           - Pop capsule_hash; load and link module
export           - Pop symbol_name, value; export from current capsule
```