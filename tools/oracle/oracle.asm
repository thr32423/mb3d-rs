; Native 32-bit test oracle for the mb3d x86 interpreter.
; Reads a 64 KiB memory image from stdin, maps it at 0x10000000, calls the
; formula code as described by the header at the start of the image and
; writes the resulting image to stdout.
;   [img+0]  entry   [img+4] eax  [img+8] edx  [img+12] ecx
;   [img+16] arg @w  [img+20] arg PIteration3D  [img+24] initial esp
;   [img+28] 1 = dIFS convention: esi=[img+32] edi=[img+36] ebx=[img+40],
;            ecx=[img+12], edx=esi+128, no stack arguments
BASE equ 0x10000000
SIZE equ 0x10000
global _start
section .bss
saved_esp: resd 1
section .text
_start:
    mov eax, 192          ; mmap2
    mov ebx, BASE
    mov ecx, SIZE
    mov edx, 7            ; rwx
    mov esi, 0x32         ; MAP_PRIVATE|MAP_ANON|MAP_FIXED
    mov edi, -1
    xor ebp, ebp
    int 0x80
    cmp eax, BASE
    jne fail
    xor esi, esi          ; bytes read
.rd:
    mov eax, 3            ; read
    xor ebx, ebx
    lea ecx, [BASE + esi]
    mov edx, SIZE
    sub edx, esi
    jz .done
    int 0x80
    test eax, eax
    jle .done
    add esi, eax
    jmp .rd
.done:
    cmp esi, SIZE
    jne fail
    finit
    mov [saved_esp], esp
    mov esp, [BASE + 24]
    cmp dword [BASE + 28], 1
    jne .std
    mov esi, [BASE + 32]
    mov edi, [BASE + 36]
    mov ebx, [BASE + 40]
    mov ecx, [BASE + 12]
    lea edx, [esi + 128]
    xor eax, eax
    call [BASE + 0]
    jmp .back
.std:
    push dword [BASE + 16]
    push dword [BASE + 20]
    mov eax, [BASE + 4]
    mov edx, [BASE + 8]
    mov ecx, [BASE + 12]
    call [BASE + 0]
.back:
    mov esp, [saved_esp]
    xor esi, esi
.wr:
    mov eax, 4            ; write
    mov ebx, 1
    lea ecx, [BASE + esi]
    mov edx, SIZE
    sub edx, esi
    jz .exit
    int 0x80
    test eax, eax
    jle fail
    add esi, eax
    jmp .wr
.exit:
    mov eax, 1
    xor ebx, ebx
    int 0x80
fail:
    mov eax, 1
    mov ebx, 3
    int 0x80
