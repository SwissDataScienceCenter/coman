#!/usr/bin/env bash

set -euo pipefail
shopt -s lastpipe nullglob

export PATH=\"${PATH}:/usr/sbin:/sbin\"

source \"${ENROOT_LIBRARY_PATH}/common.sh\"

common::checkcmd grep sed ldd ldconfig

if [ \"${OCI_ANNOTATION_com__hooks__dcgm__enabled:-}\" != \"true\" ]; then
    exit 0
fi

# Mounting the specified DCGM libraries and directories explicitly
cat << EOF | enroot-mount --root \"${ENROOT_ROOTFS}\" -
/usr/local/dcgm /usr/local/dcgm none x-create=dir,bind,ro,nosuid,nodev,private
/usr/lib64/libnvperf_dcgm_host.so /usr/lib64/libnvperf_dcgm_host.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulesysmon.so.3.3.6 /usr/lib64/libdcgmmodulesysmon.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulesysmon.so.3 /usr/lib64/libdcgmmodulesysmon.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulesysmon.so /usr/lib64/libdcgmmodulesysmon.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleprofiling.so.3.3.6 /usr/lib64/libdcgmmoduleprofiling.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleprofiling.so.3 /usr/lib64/libdcgmmoduleprofiling.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleprofiling.so /usr/lib64/libdcgmmoduleprofiling.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulepolicy.so.3.3.6 /usr/lib64/libdcgmmodulepolicy.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulepolicy.so.3 /usr/lib64/libdcgmmodulepolicy.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulepolicy.so /usr/lib64/libdcgmmodulepolicy.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulenvswitch.so.3.3.6 /usr/lib64/libdcgmmodulenvswitch.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulenvswitch.so.3 /usr/lib64/libdcgmmodulenvswitch.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulenvswitch.so /usr/lib64/libdcgmmodulenvswitch.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleintrospect.so.3.3.6 /usr/lib64/libdcgmmoduleintrospect.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleintrospect.so.3 /usr/lib64/libdcgmmoduleintrospect.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleintrospect.so /usr/lib64/libdcgmmoduleintrospect.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulehealth.so.3.3.6 /usr/lib64/libdcgmmodulehealth.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulehealth.so.3 /usr/lib64/libdcgmmodulehealth.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulehealth.so /usr/lib64/libdcgmmodulehealth.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulediag.so.3.3.6 /usr/lib64/libdcgmmodulediag.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulediag.so.3 /usr/lib64/libdcgmmodulediag.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmodulediag.so /usr/lib64/libdcgmmodulediag.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleconfig.so.3.3.6 /usr/lib64/libdcgmmoduleconfig.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleconfig.so.3 /usr/lib64/libdcgmmoduleconfig.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgmmoduleconfig.so /usr/lib64/libdcgmmoduleconfig.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgm_stub.a /usr/lib64/libdcgm_stub.a none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgm_cublas_proxy12.so /usr/lib64/libdcgm_cublas_proxy12.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgm_cublas_proxy11.so /usr/lib64/libdcgm_cublas_proxy11.so none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgm.so.3.3.6 /usr/lib64/libdcgm.so.3.3.6 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgm.so.3 /usr/lib64/libdcgm.so.3 none x-create=file,bind,ro,nosuid,nodev,private
/usr/lib64/libdcgm.so /usr/lib64/libdcgm.so none x-create=file,bind,ro,nosuid,nodev,private
EOF

# Refresh the dynamic linker cache to include newly mounted libs
cat << EOF > \"${ENROOT_ROOTFS}/etc/ld.so.conf.d/enroot-dcgm-hook.conf\"
/lib64
/usr/lib64
EOF

if ! ${ldconfig:-ldconfig} -r \"${ENROOT_ROOTFS}\" >> \"${ENROOT_ROOTFS}/dcgm-hook.log\" 2>&1; then
    common::err \"Failed to refresh the dynamic linker cache\"
fi
