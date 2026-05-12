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
  totpCode: json['totpCode'] as String?,
  backupCode: json['backupCode'] as String?,
  sessionTimeoutHours: (json['sessionTimeoutHours'] as num?)?.toInt(),
);

Map<String, dynamic> _$LoginRequestToJson(LoginRequest instance) =>
    <String, dynamic>{
      'email': instance.email,
      'password': instance.password,
      'totpCode': instance.totpCode,
      'backupCode': instance.backupCode,
      'sessionTimeoutHours': instance.sessionTimeoutHours,
    };

LoginResponse _$LoginResponseFromJson(Map<String, dynamic> json) =>
    LoginResponse(
      success: json['success'] as bool?,
      twoFactorEnabled: json['twoFactorEnabled'] as bool?,
      sessionId: json['sessionId'] as String?,
      userEmail: json['userEmail'] as String?,
      walletAddress: json['walletAddress'] as String?,
      curveType: json['curveType'] as String?,
      encryptedPrivateKey: json['encryptedPrivateKey'] as String?,
      expiresAt: json['expiresAt'] as String?,
    );

Map<String, dynamic> _$LoginResponseToJson(LoginResponse instance) =>
    <String, dynamic>{
      'success': instance.success,
      'twoFactorEnabled': instance.twoFactorEnabled,
      'sessionId': instance.sessionId,
      'userEmail': instance.userEmail,
      'walletAddress': instance.walletAddress,
      'curveType': instance.curveType,
      'encryptedPrivateKey': instance.encryptedPrivateKey,
      'expiresAt': instance.expiresAt,
    };

TwoFactorSetupRequest _$TwoFactorSetupRequestFromJson(
  Map<String, dynamic> json,
) => TwoFactorSetupRequest(
  email: json['email'] as String,
  password: json['password'] as String,
);

Map<String, dynamic> _$TwoFactorSetupRequestToJson(
  TwoFactorSetupRequest instance,
) => <String, dynamic>{'email': instance.email, 'password': instance.password};

TwoFactorSetupResponse _$TwoFactorSetupResponseFromJson(
  Map<String, dynamic> json,
) => TwoFactorSetupResponse(
  success: json['success'] as bool?,
  secret: json['secret'] as String?,
  otpauthUrl: json['otpauthUrl'] as String?,
  qrCodeSvg: json['qrCodeSvg'] as String?,
  backupCodes: (json['backupCodes'] as List<dynamic>?)
      ?.map((e) => e as String)
      .toList(),
  message: json['message'] as String?,
);

Map<String, dynamic> _$TwoFactorSetupResponseToJson(
  TwoFactorSetupResponse instance,
) => <String, dynamic>{
  'success': instance.success,
  'secret': instance.secret,
  'otpauthUrl': instance.otpauthUrl,
  'qrCodeSvg': instance.qrCodeSvg,
  'backupCodes': instance.backupCodes,
  'message': instance.message,
};

Enable2faRequest _$Enable2faRequestFromJson(Map<String, dynamic> json) =>
    Enable2faRequest(
      email: json['email'] as String,
      password: json['password'] as String,
      code: json['code'] as String,
    );

Map<String, dynamic> _$Enable2faRequestToJson(Enable2faRequest instance) =>
    <String, dynamic>{
      'email': instance.email,
      'password': instance.password,
      'code': instance.code,
    };

Disable2faRequest _$Disable2faRequestFromJson(Map<String, dynamic> json) =>
    Disable2faRequest(
      email: json['email'] as String,
      password: json['password'] as String,
    );

Map<String, dynamic> _$Disable2faRequestToJson(Disable2faRequest instance) =>
    <String, dynamic>{'email': instance.email, 'password': instance.password};

Verify2faRequest _$Verify2faRequestFromJson(Map<String, dynamic> json) =>
    Verify2faRequest(
      email: json['email'] as String,
      code: json['code'] as String,
    );

Map<String, dynamic> _$Verify2faRequestToJson(Verify2faRequest instance) =>
    <String, dynamic>{'email': instance.email, 'code': instance.code};

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
