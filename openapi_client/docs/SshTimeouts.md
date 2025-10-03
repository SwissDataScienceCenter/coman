# SshTimeouts

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**command_execution** | Option<**i32**> | Timeout (seconds) for executing commands over SSH. | [optional][default to 5]
**connection** | Option<**i32**> | Timeout (seconds) for initial SSH connection. | [optional][default to 5]
**idle_timeout** | Option<**i32**> | Max idle time (seconds) before disconnecting. | [optional][default to 60]
**keep_alive** | Option<**i32**> | Interval (seconds) for sending keep-alive messages. | [optional][default to 5]
**login** | Option<**i32**> | Timeout (seconds) for SSH login/auth. | [optional][default to 5]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


