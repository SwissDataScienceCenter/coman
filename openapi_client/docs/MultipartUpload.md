# MultipartUpload

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**max_part_size** | Option<**i32**> | Maximum size (in bytes) for multipart data transfers. Default is 2 GB. | [optional][default to 2147483648]
**parallel_runs** | Option<**i32**> | Number of parts to upload in parallel to the staging area. | [optional][default to 3]
**tmp_folder** | Option<**String**> | Temporary folder used for storing split parts during upload. | [optional][default to tmp]
**use_split** | Option<**bool**> | Enable or disable splitting large files into parts when uploading the file to the staging area. | [optional][default to false]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


