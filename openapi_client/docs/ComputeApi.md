# \ComputeApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**attach_compute_system_name_jobs_job_id_attach_put**](ComputeApi.md#attach_compute_system_name_jobs_job_id_attach_put) | **PUT** /compute/{system_name}/jobs/{job_id}/attach | Attach
[**delete_job_cancel_compute_system_name_jobs_job_id_delete**](ComputeApi.md#delete_job_cancel_compute_system_name_jobs_job_id_delete) | **DELETE** /compute/{system_name}/jobs/{job_id} | Delete Job Cancel
[**get_job_compute_system_name_jobs_job_id_get**](ComputeApi.md#get_job_compute_system_name_jobs_job_id_get) | **GET** /compute/{system_name}/jobs/{job_id} | Get Job
[**get_job_metadata_compute_system_name_jobs_job_id_metadata_get**](ComputeApi.md#get_job_metadata_compute_system_name_jobs_job_id_metadata_get) | **GET** /compute/{system_name}/jobs/{job_id}/metadata | Get Job Metadata
[**get_jobs_compute_system_name_jobs_get**](ComputeApi.md#get_jobs_compute_system_name_jobs_get) | **GET** /compute/{system_name}/jobs | Get Jobs
[**post_job_submit_compute_system_name_jobs_post**](ComputeApi.md#post_job_submit_compute_system_name_jobs_post) | **POST** /compute/{system_name}/jobs | Post Job Submit



## attach_compute_system_name_jobs_job_id_attach_put

> attach_compute_system_name_jobs_job_id_attach_put(job_id, system_name, post_job_attach_request)
Attach

Attach a procces to a job by `{job_id}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**job_id** | **String** | Job id | [required] |
**system_name** | **String** |  | [required] |
**post_job_attach_request** | [**PostJobAttachRequest**](PostJobAttachRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_job_cancel_compute_system_name_jobs_job_id_delete

> delete_job_cancel_compute_system_name_jobs_job_id_delete(job_id, system_name)
Delete Job Cancel

Cancel a job

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**job_id** | **String** | Job id | [required] |
**system_name** | **String** |  | [required] |

### Return type

 (empty response body)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_job_compute_system_name_jobs_job_id_get

> models::GetJobResponse get_job_compute_system_name_jobs_job_id_get(job_id, system_name)
Get Job

Get status of a job by `{job_id}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**job_id** | **String** | Job id | [required] |
**system_name** | **String** |  | [required] |

### Return type

[**models::GetJobResponse**](GetJobResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_job_metadata_compute_system_name_jobs_job_id_metadata_get

> models::GetJobMetadataResponse get_job_metadata_compute_system_name_jobs_job_id_metadata_get(job_id, system_name)
Get Job Metadata

Get metadata of a job by `{job_id}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**job_id** | **String** | Job id | [required] |
**system_name** | **String** |  | [required] |

### Return type

[**models::GetJobMetadataResponse**](GetJobMetadataResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_jobs_compute_system_name_jobs_get

> models::GetJobResponse get_jobs_compute_system_name_jobs_get(system_name, allusers)
Get Jobs

Get status of all jobs

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**system_name** | **String** |  | [required] |
**allusers** | Option<**bool**> | If set to `true` returns all jobs visible by the current user, otherwise only the current user owned jobs |  |[default to false]

### Return type

[**models::GetJobResponse**](GetJobResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## post_job_submit_compute_system_name_jobs_post

> models::PostJobSubmissionResponse post_job_submit_compute_system_name_jobs_post(system_name, post_job_submit_request)
Post Job Submit

Submit a new job

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**system_name** | **String** |  | [required] |
**post_job_submit_request** | [**PostJobSubmitRequest**](PostJobSubmitRequest.md) |  | [required] |

### Return type

[**models::PostJobSubmissionResponse**](PostJobSubmissionResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

