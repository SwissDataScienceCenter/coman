# S3DataTransfer

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**access_key_id** | **String** | Access key ID for S3-compatible storage. | 
**bucket_lifecycle_configuration** | Option<[**models::BucketLifecycleConfiguration**](BucketLifecycleConfiguration.md)> | Lifecycle policy settings for auto-deleting files after a given number of days. | [optional]
**multipart** | Option<[**models::MultipartUpload**](MultipartUpload.md)> | Settings for multipart upload, including chunk size and concurrency. | [optional]
**name** | **String** | Name identifier for the storage. | 
**private_url** | **String** | Private/internal endpoint URL for the storage. | 
**probing** | Option<[**models::Probing**](Probing.md)> |  | [optional]
**public_url** | **String** | Public/external URL for the storage. | 
**region** | **String** | Region of the storage bucket. | 
**secret_access_key** | **String** | Secret access key for storage. You can give directly the content or the file path using `'secret_file:/path/to/file'`. | 
**service_type** | **String** |  | 
**services_health** | Option<[**Vec<models::S3DataTransferServicesHealthInner>**](S3DataTransfer_servicesHealth_inner.md)> |  | [optional]
**tenant** | Option<**String**> |  | [optional]
**ttl** | **i32** | Time-to-live (in seconds) for generated URLs. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


