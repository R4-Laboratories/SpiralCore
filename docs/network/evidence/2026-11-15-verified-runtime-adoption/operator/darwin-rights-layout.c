#include <stddef.h>
#include <stdio.h>
#include <sys/socket.h>

struct sc_rights_control {
    _Alignas(16) unsigned char rights[CMSG_SPACE(512 * sizeof(int))];
};

int main(void) {
    printf("cmsghdr_size=%zu cmsghdr_align=%zu rights_base=%zu int_size=%zu\n",
           sizeof(struct cmsghdr), _Alignof(struct cmsghdr), (size_t)CMSG_LEN(0), sizeof(int));
    printf("control_bytes=%zu control_size=%zu control_align=%zu rights_offset=%zu\n",
           (size_t)CMSG_SPACE(512 * sizeof(int)), sizeof(struct sc_rights_control),
           _Alignof(struct sc_rights_control), offsetof(struct sc_rights_control, rights));
    return 0;
}
