# JobModel

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account** | Option<**String**> |  | [optional]
**allocation_nodes** | **i32** |  | 
**cluster** | **String** |  | 
**group** | Option<**String**> |  | [optional]
**job_id** | **i32** |  | 
**kill_request_user** | Option<**String**> |  | [optional]
**name** | **String** |  | 
**nodes** | **String** |  | 
**partition** | **String** |  | 
**priority** | Option<**i32**> |  | [optional]
**status** | [**models::JobStatus**](JobStatus.md) |  | 
**tasks** | Option<[**Vec<models::JobTask>**](JobTask.md)> |  | [optional]
**time** | [**models::JobTime**](JobTime.md) |  | 
**user** | Option<**String**> |  | 
**working_directory** | **String** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


