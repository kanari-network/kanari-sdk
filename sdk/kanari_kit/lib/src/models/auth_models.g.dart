// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'auth_models.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RegisterRequest _$RegisterRequestFromJson(Map<String, dynamic> json) =>
    RegisterRequest(
      email: json['email'] as String,
      password: json['password'] as String,
      curveType: json['curveType'] as String?,
    );

Map<String, dynamic> _$RegisterRequestToJson(RegisterRequest instance) =>
    <String, dynamic>{
      'email': instance.email,
      'password': instance.password,
      'curveType': instance.curveType,
    };

RegisterResponse _$RegisterResponseFromJson(Map<String, dynamic> json) =>
    RegisterResponse(
      success: json['success'] as bool?,
      walletAddress: json['walletAddress'] as String?,
      message: json['message'] as String?,
    );

Map<String, dynamic> _$RegisterResponseToJson(RegisterResponse instance) =>
    <String, dynamic>{
      'success': instance.success,
      'walletAddress': instance.walletAddress,
      'message': instance.message,
    };

LoginRequest _$LoginRequestFromJson(Map<String, dynamic> json) => LoginRequest(
  email: json['email'] as String,
  password: json['password'] as String,
  sessionTimeoutHours: (json['sessionTimeoutHours'] as num?)?.toInt(),
);

Map<String, dynamic> _$LoginRequestToJson(LoginRequest instance) =>
    <String, dynamic>{
      'email': instance.email,
      'password': instance.password,
      'sessionTimeoutHours': instance.sessionTimeoutHours,
    };

LoginResponse _$LoginResponseFromJson(Map<String, dynamic> json) =>
    LoginResponse(
      success: json['success'] as bool?,
      sessionId: json['sessionId'] as String?,
      userEmail: json['userEmail'] as String?,
      walletAddress: json['walletAddress'] as String?,
      expiresAt: json['expiresAt'] as String?,
    );

Map<String, dynamic> _$LoginResponseToJson(LoginResponse instance) =>
    <String, dynamic>{
      'success': instance.success,
      'sessionId': instance.sessionId,
      'userEmail': instance.userEmail,
      'walletAddress': instance.walletAddress,
      'expiresAt': instance.expiresAt,
    };

LogoutRequest _$LogoutRequestFromJson(Map<String, dynamic> json) =>
    LogoutRequest(sessionId: json['sessionId'] as String);

Map<String, dynamic> _$LogoutRequestToJson(LogoutRequest instance) =>
    <String, dynamic>{'sessionId': instance.sessionId};

LogoutAllRequest _$LogoutAllRequestFromJson(Map<String, dynamic> json) =>
    LogoutAllRequest(email: json['email'] as String);

Map<String, dynamic> _$LogoutAllRequestToJson(LogoutAllRequest instance) =>
    <String, dynamic>{'email': instance.email};

ChangePasswordRequest _$ChangePasswordRequestFromJson(
  Map<String, dynamic> json,
) => ChangePasswordRequest(
  email: json['email'] as String,
  oldPassword: json['oldPassword'] as String,
  newPassword: json['newPassword'] as String,
);

Map<String, dynamic> _$ChangePasswordRequestToJson(
  ChangePasswordRequest instance,
) => <String, dynamic>{
  'email': instance.email,
  'oldPassword': instance.oldPassword,
  'newPassword': instance.newPassword,
};

DeleteAccountRequest _$DeleteAccountRequestFromJson(
  Map<String, dynamic> json,
) => DeleteAccountRequest(
  email: json['email'] as String,
  password: json['password'] as String,
);

Map<String, dynamic> _$DeleteAccountRequestToJson(
  DeleteAccountRequest instance,
) => <String, dynamic>{'email': instance.email, 'password': instance.password};

SignTransferRequest _$SignTransferRequestFromJson(Map<String, dynamic> json) =>
    SignTransferRequest(
      sessionId: json['sessionId'] as String,
      recipient: json['recipient'] as String,
      amount: (json['amount'] as num).toInt(),
      gasLimit: (json['gasLimit'] as num?)?.toInt(),
      gasPrice: (json['gasPrice'] as num?)?.toInt(),
    );

Map<String, dynamic> _$SignTransferRequestToJson(
  SignTransferRequest instance,
) => <String, dynamic>{
  'sessionId': instance.sessionId,
  'recipient': instance.recipient,
  'amount': instance.amount,
  'gasLimit': instance.gasLimit,
  'gasPrice': instance.gasPrice,
};

ValidateSessionResponse _$ValidateSessionResponseFromJson(
  Map<String, dynamic> json,
) => ValidateSessionResponse(
  valid: json['valid'] as bool,
  sessionId: json['sessionId'] as String,
);

Map<String, dynamic> _$ValidateSessionResponseToJson(
  ValidateSessionResponse instance,
) => <String, dynamic>{
  'valid': instance.valid,
  'sessionId': instance.sessionId,
};

UserInfoResponse _$UserInfoResponseFromJson(Map<String, dynamic> json) =>
    UserInfoResponse(
      email: json['email'] as String,
      walletAddress: json['walletAddress'] as String,
      createdAt: json['createdAt'] as String,
      lastLogin: json['lastLogin'] as String?,
    );

Map<String, dynamic> _$UserInfoResponseToJson(UserInfoResponse instance) =>
    <String, dynamic>{
      'email': instance.email,
      'walletAddress': instance.walletAddress,
      'createdAt': instance.createdAt,
      'lastLogin': instance.lastLogin,
    };

ListUsersResponse _$ListUsersResponseFromJson(Map<String, dynamic> json) =>
    ListUsersResponse(
      users: (json['users'] as List<dynamic>).map((e) => e as String).toList(),
      count: (json['count'] as num).toInt(),
    );

Map<String, dynamic> _$ListUsersResponseToJson(ListUsersResponse instance) =>
    <String, dynamic>{'users': instance.users, 'count': instance.count};

ApiResponse<T> _$ApiResponseFromJson<T>(
  Map<String, dynamic> json,
  T Function(Object? json) fromJsonT,
) => ApiResponse<T>(
  success: json['success'] as bool,
  data: _$nullableGenericFromJson(json['data'], fromJsonT),
  error: json['error'] as String?,
);

Map<String, dynamic> _$ApiResponseToJson<T>(
  ApiResponse<T> instance,
  Object? Function(T value) toJsonT,
) => <String, dynamic>{
  'success': instance.success,
  'data': _$nullableGenericToJson(instance.data, toJsonT),
  'error': instance.error,
};

T? _$nullableGenericFromJson<T>(
  Object? input,
  T Function(Object? json) fromJson,
) => input == null ? null : fromJson(input);

Object? _$nullableGenericToJson<T>(
  T? input,
  Object? Function(T value) toJson,
) => input == null ? null : toJson(input);
