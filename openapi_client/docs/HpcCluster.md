# HpcCluster

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**datatransfer_jobs_directives** | Option<**Vec<String>**> | Custom scheduler flags passed to data transfer jobs (e.g. `-pxfer` for a dedicated partition). | [optional]
**file_systems** | Option<[**Vec<models::FileSystem>**](FileSystem.md)> | List of mounted file systems on the cluster, such as scratch or home directories. | [optional]
**name** | **String** | Unique name for the cluster. This field is case insensitive. | 
**probing** | [**models::Probing**](Probing.md) | Probing configuration for monitoring the cluster. | 
**scheduler** | [**models::Scheduler**](Scheduler.md) | Job scheduler configuration. | 
**services_health** | Option<[**Vec<models::HpcClusterServicesHealthInner>**](HPCCluster_servicesHealth_inner.md)> |  | [optional]
**ssh** | [**models::SshClientPool**](SSHClientPool.md) | SSH configuration for accessing the cluster nodes. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


