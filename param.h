// maximum number of processes
#define NPROC             64
// The size of per-process kernel stack.
#define KERNEL_STACK_SIZE 16384
// maximum number of CPUs
#define NCPU              8
// open files per process
#define NOFILE            16
// open files per system
#define NFILE             100
// maximum number of active i-nodes
#define NINODE            50
// maximum major device number
#define NDEV              10
// device number of file system root disk
#define ROOTDEV           1
// max exec arguments
#define MAXARG            32
// max # of blocks any FS op writes
#define MAXOPBLOCKS       10
// max data blocks in on-disk log
#define LOGSIZE           (MAXOPBLOCKS*3)
// size of disk block cache
#define NBUF              (MAXOPBLOCKS*3)
#ifdef PDX_XV6
#define FSSIZE       2000  // size of file system in blocks
#else
#define FSSIZE       1000  // size of file system in blocks
#endif // PDX_XV6
