# \StatusApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_liveness_status_liveness_get**](StatusApi.md#get_liveness_status_liveness_get) | **GET** /status/liveness/ | Get Liveness
[**get_system_nodes_status_system_name_nodes_get**](StatusApi.md#get_system_nodes_status_system_name_nodes_get) | **GET** /status/{system_name}/nodes | Get System Nodes
[**get_system_partitions_status_system_name_partitions_get**](StatusApi.md#get_system_partitions_status_system_name_partitions_get) | **GET** /status/{system_name}/partitions | Get System Partitions
[**get_system_reservations_status_system_name_reservations_get**](StatusApi.md#get_system_reservations_status_system_name_reservations_get) | **GET** /status/{system_name}/reservations | Get System Reservations
[**get_systems_status_systems_get**](StatusApi.md#get_systems_status_systems_get) | **GET** /status/systems | Get Systems
[**get_userinfo_status_system_name_userinfo_get**](StatusApi.md#get_userinfo_status_system_name_userinfo_get) | **GET** /status/{system_name}/userinfo | Get Userinfo



## get_liveness_status_liveness_get

> models::GetLiveness get_liveness_status_liveness_get()
Get Liveness

Get liveness status of FirecREST

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::GetLiveness**](GetLiveness.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_system_nodes_status_system_name_nodes_get

> models::GetNodesResponse get_system_nodes_status_system_name_nodes_get(system_name)
Get System Nodes

Get the list of nodes of a `{system_name}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**system_name** | **String** |  | [required] |

### Return type

[**models::GetNodesResponse**](GetNodesResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_system_partitions_status_system_name_partitions_get

> models::GetPartitionsResponse get_system_partitions_status_system_name_partitions_get(system_name)
Get System Partitions

Get the list of partitions of a `{system_name}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**system_name** | **String** |  | [required] |

### Return type

[**models::GetPartitionsResponse**](GetPartitionsResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_system_reservations_status_system_name_reservations_get

> models::GetReservationsResponse get_system_reservations_status_system_name_reservations_get(system_name)
Get System Reservations

Get the list of reservations of a `{system_name}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**system_name** | **String** |  | [required] |

### Return type

[**models::GetReservationsResponse**](GetReservationsResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_systems_status_systems_get

> models::GetSystemsResponse get_systems_status_systems_get()
Get Systems

Get the list of systems and health status

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::GetSystemsResponse**](GetSystemsResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_userinfo_status_system_name_userinfo_get

> models::UserInfoResponse get_userinfo_status_system_name_userinfo_get(system_name)
Get Userinfo

Get current user information on a `{system_name}`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**system_name** | **String** |  | [required] |

### Return type

[**models::UserInfoResponse**](UserInfoResponse.md)

### Authorization

[HTTPBearer](../README.md#HTTPBearer), [APIAuthDependency](../README.md#APIAuthDependency)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

