MEMORY
{
    MEM : org = 0, len = 96k
}

SECTIONS
{
    . = 0;
    _text = .;

    .program_ptr : ALIGN(4)
    {
        . = ALIGN(4);
        *(.program_ptr)
    } > MEM

    .init_fun : ALIGN(4)
    {
        . = ALIGN(4);
        *(.init_fun)
    } > MEM

    .data : ALIGN(4)
    {
        . = ALIGN(4);
        *(.data)
        *(.data*)
    } > MEM

    .bss : ALIGN(4)
    {
        . = ALIGN(4);
        *(.bss)
        *(.bss*)
        *(COMMON)
    } > MEM

    .got : ALIGN(4)
    {
        . = ALIGN(4);
        *(.got*)
        . = ALIGN(4);
    } > MEM

    .text : ALIGN(16) SUBALIGN(16)
    {
        *(.text)
        *(.text*)
    } > MEM

    .rodata : ALIGN(4)
    {
        *(.rodata)
        *(.rodata.*)
    } > MEM

    /DISCARD/ :
    {
        *(.ARM.exidx*)
        *(.ARM.extab*)
        *(.comment*)
        *(.note*)
    }
}
