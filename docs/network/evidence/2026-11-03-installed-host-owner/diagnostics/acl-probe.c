#include <sys/types.h>
#include <sys/stat.h>
#include <sys/acl.h>
#include <fcntl.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
int main(int argc, char **argv) {
 if (argc != 3) return 2;
 int fd = open(argv[2], O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
 if (fd < 0) return 3;
 struct stat before, after, sx;
 if (fstat(fd, &before)) return 4;
 errno = 0;
 acl_t acl = acl_get_fd_np(fd, ACL_TYPE_EXTENDED);
 int get_errno = errno;
 int entry = -2, entry_errno = 0;
 if (acl) { acl_entry_t e; errno = 0; entry = acl_get_entry(acl, ACL_FIRST_ENTRY, &e); entry_errno = errno; acl_free(acl); }
 filesec_t sec = filesec_init();
 if (!sec) return 5;
 errno = 0; int statx = fstatx_np(fd, &sx, sec); int statx_errno = errno;
 int valid = -1; errno = 0; int query = filesec_query_property(sec, FILESEC_ACL, &valid); int query_errno = errno;
 acl_t property = NULL; errno = 0; int prop = filesec_get_property(sec, FILESEC_ACL, &property); int prop_errno = errno;
 if (property) acl_free(property);
 filesec_free(sec);
 if (fstat(fd, &after)) return 6;
 printf("%s type=%s mode=%o stable_fd=%d acl_nonnull=%d acl_errno=%d entry=%d entry_errno=%d fstatx=%d fstatx_errno=%d query=%d valid=%d query_errno=%d property=%d property_errno=%d\n", argv[1], S_ISDIR(before.st_mode) ? "dir" : "file", before.st_mode & 07777, before.st_dev == after.st_dev && before.st_ino == after.st_ino, acl != NULL, get_errno, entry, entry_errno, statx, statx_errno, query, valid, query_errno, prop, prop_errno);
 close(fd); return 0;
}
