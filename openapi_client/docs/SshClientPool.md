# SshClientPool

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**host** | **String** | SSH target hostname. | 
**max_clients** | Option<**i32**> | Maximum number of concurrent SSH clients. | [optional][default to 100]
**port** | **i32** | SSH port. | 
**proxy_host** | Option<**String**> |  | [optional]
**proxy_port** | Option<**i32**> |  | [optional]
**timeout** | Option<[**models::SshTimeouts**](SSHTimeouts.md)> | SSH timeout settings. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


