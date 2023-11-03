# Allocation
there's 5 allocators, which is too many but memory allocation algorithms are neat so i'm doing it anyways

bump_alloc
- more like a set of pointer manipulation functions than an allocator
- splits a buffer into multiple allocations, handles alignment, etc.
- there is no global bump_alloc, make one for each buffer you bump-allocate from
- used in other allocators
- no dependencies

frame_alloc
- produces physical addresses of page frames
  - an order-0 frame is 4096 bytes
  - an order-n frame is 2^n contiguous order-0 frames
    - only 2^2n is supported currently
- no dependencies, allocates all needed memory at boot

page_alloc
- produces virtual addresses of pages and page tables
- they cannot be deallocated, use sparingly
- used mostly to allocate page tables for more complex allocators
- depends on frame_alloc to back the allocated tables
bonus thoughts:
  - this actually needs to solve 2 different problems:
    allocating regions in the virtual address space for page tables
    and mapping those regions (which requires page tables)

slab_alloc
- produces virtual addresses of small objects
- depends on page_alloc to allocate slabs
  - splits an l2 table into chunks, each chunk is a slab
- basic structure is similar to mimalloc
  - could add concurrency features like mimalloc too

the address space thing
- this needs a name
- same mechanism as user-space process address spaces
- used for big allocations
- depends on page_alloc and slab_alloc
  - splits an l1 table into 2-page chunks (page table + virt addrs of sub-tables)
  - slab alloc for various small objects involved in bookkeeping


page_alloc and the address space thing can use a shared tool (page table manager) to do the mapping
it's only the allocation patterns that are different