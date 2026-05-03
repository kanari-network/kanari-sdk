import 'package:json_annotation/json_annotation.dart';

part 'auth_models.g.dart';

/// Registration request for Kanari Auth API
@JsonSerializable()
class RegisterRequest {
  final String email;
  final String password;
  final String? curveType;

  RegisterRequest({
    required this.email,
    required this.password,
    this.curveType,
  });

  factory RegisterRequest.fromJson(Map<String, dynamic> json) =>
      _$RegisterRequestFromJson(json);

  Map<String, dynamic> toJson() => _$RegisterRequestToJson(this);
}

/// Registration response
@JsonSerializable()
class RegisterResponse {
  final bool? success;
  final String? walletAddress;
  final String? message;

  RegisterResponse({this.success, this.walletAddress, this.message});

  factory RegisterResponse.fromJson(Map<String, dynamic> json) =>
      _$RegisterResponseFromJson(json);

  Map<String, dynamic> toJson() => _$RegisterResponseToJson(this);
}

/// Login request
@JsonSerializable()
class LoginRequest {
  final String email;
  final String password;
  final String? totpCode;
  final String? backupCode;
  final int? sessionTimeoutHours;

  LoginRequest({
    required this.email,
    required this.password,
    this.totpCode,
    this.backupCode,
    this.sessionTimeoutHours,
  });

  factory LoginRequest.fromJson(Map<String, dynamic> json) =>
      _$LoginRequestFromJson(json);

  Map<String, dynamic> toJson() => _$LoginRequestToJson(this);
}

/// Login response with session information
@JsonSerializable()
class LoginResponse {
  final bool? success;
  final bool? twoFactorEnabled;
  final String? sessionId;
  final String? userEmail;
  final String? walletAddress;
  final String? curveType;
  final String? encryptedPrivateKey;
  final String? expiresAt;

  LoginResponse({
    this.success,
    this.twoFactorEnabled,
    this.sessionId,
    this.userEmail,
    this.walletAddress,
    this.curveType,
    this.encryptedPrivateKey,
    this.expiresAt,
  });

  factory LoginResponse.fromJson(Map<String, dynamic> json) =>
      _$LoginResponseFromJson(json);

  Map<String, dynamic> toJson() => _$LoginResponseToJson(this);
}

/// Two-factor setup request
@JsonSerializable()
class TwoFactorSetupRequest {
  final String email;
  final String password;

  TwoFactorSetupRequest({required this.email, required this.password});

  factory TwoFactorSetupRequest.fromJson(Map<String, dynamic> json) =>
      _$TwoFactorSetupRequestFromJson(json);

  Map<String, dynamic> toJson() => _$TwoFactorSetupRequestToJson(this);
}

/// Two-factor setup response
@JsonSerializable()
class TwoFactorSetupResponse {
  final bool? success;
  final String? secret;
  final String? otpauthUrl;
  final String? qrCodeSvg;
  final List<String>? backupCodes;
  final String? message;

  TwoFactorSetupResponse({
    this.success,
    this.secret,
    this.otpauthUrl,
    this.qrCodeSvg,
    this.backupCodes,
    this.message,
  });

  factory TwoFactorSetupResponse.fromJson(Map<String, dynamic> json) =>
      _$TwoFactorSetupResponseFromJson(json);

  Map<String, dynamic> toJson() => _$TwoFactorSetupResponseToJson(this);
}

/// Enable two-factor request
@JsonSerializable()
class Enable2faRequest {
  final String email;
  final String password;
  final String code;

  Enable2faRequest({
    required this.email,
    required this.password,
    required this.code,
  });

  factory Enable2faRequest.fromJson(Map<String, dynamic> json) =>
      _$Enable2faRequestFromJson(json);

  Map<String, dynamic> toJson() => _$Enable2faRequestToJson(this);
}

/// Disable two-factor request
@JsonSerializable()
class Disable2faRequest {
  final String email;
  final String password;

  Disable2faRequest({required this.email, required this.password});

  factory Disable2faRequest.fromJson(Map<String, dynamic> json) =>
      _$Disable2faRequestFromJson(json);

  Map<String, dynamic> toJson() => _$Disable2faRequestToJson(this);
}

/// Verify two-factor code request
@JsonSerializable()
class Verify2faRequest {
  final String email;
  final String code;

  Verify2faRequest({required this.email, required this.code});

  factory Verify2faRequest.fromJson(Map<String, dynamic> json) =>
      _$Verify2faRequestFromJson(json);

  Map<String, dynamic> toJson() => _$Verify2faRequestToJson(this);
}

/// Logout request
@JsonSerializable()
class LogoutRequest {
  final String sessionId;

  LogoutRequest({required this.sessionId});

  factory LogoutRequest.fromJson(Map<String, dynamic> json) =>
      _$LogoutRequestFromJson(json);

  Map<String, dynamic> toJson() => _$LogoutRequestToJson(this);
}

/// Logout all sessions request
@JsonSerializable()
class LogoutAllRequest {
  final String email;

  LogoutAllRequest({required this.email});

  factory LogoutAllRequest.fromJson(Map<String, dynamic> json) =>
      _$LogoutAllRequestFromJson(json);

  Map<String, dynamic> toJson() => _$LogoutAllRequestToJson(this);
}

/// Change password request
@JsonSerializable()
class ChangePasswordRequest {
  final String email;
  final String oldPassword;
  final String newPassword;

  ChangePasswordRequest({
    required this.email,
    required this.oldPassword,
    required this.newPassword,
  });

  factory ChangePasswordRequest.fromJson(Map<String, dynamic> json) =>
      _$ChangePasswordRequestFromJson(json);

  Map<String, dynamic> toJson() => _$ChangePasswordRequestToJson(this);
}

/// Delete account request
@JsonSerializable()
class DeleteAccountRequest {
  final String email;
  final String password;

  DeleteAccountRequest({required this.email, required this.password});

  factory DeleteAccountRequest.fromJson(Map<String, dynamic> json) =>
      _$DeleteAccountRequestFromJson(json);

  Map<String, dynamic> toJson() => _$DeleteAccountRequestToJson(this);
}

/// Sign transfer request
@JsonSerializable()
class SignTransferRequest {
  final String sessionId;
  final String recipient;
  final int amount;
  final int? gasLimit;
  final int? gasPrice;

  SignTransferRequest({
    required this.sessionId,
    required this.recipient,
    required this.amount,
    this.gasLimit,
    this.gasPrice,
  });

  factory SignTransferRequest.fromJson(Map<String, dynamic> json) =>
      _$SignTransferRequestFromJson(json);

  Map<String, dynamic> toJson() => _$SignTransferRequestToJson(this);
}

/// Session validation response
@JsonSerializable()
class ValidateSessionResponse {
  final bool valid;
  final String sessionId;

  ValidateSessionResponse({required this.valid, required this.sessionId});

  factory ValidateSessionResponse.fromJson(Map<String, dynamic> json) =>
      _$ValidateSessionResponseFromJson(json);

  Map<String, dynamic> toJson() => _$ValidateSessionResponseToJson(this);
}

/// User info response
@JsonSerializable()
class UserInfoResponse {
  final String email;
  final String walletAddress;
  final String createdAt;
  final String? lastLogin;

  UserInfoResponse({
    required this.email,
    required this.walletAddress,
    required this.createdAt,
    this.lastLogin,
  });

  factory UserInfoResponse.fromJson(Map<String, dynamic> json) =>
      _$UserInfoResponseFromJson(json);

  Map<String, dynamic> toJson() => _$UserInfoResponseToJson(this);
}

/// List users response
@JsonSerializable()
class ListUsersResponse {
  final List<String> users;
  final int count;

  ListUsersResponse({required this.users, required this.count});

  factory ListUsersResponse.fromJson(Map<String, dynamic> json) =>
      _$ListUsersResponseFromJson(json);

  Map<String, dynamic> toJson() => _$ListUsersResponseToJson(this);
}

/// Generic API response wrapper
@JsonSerializable(genericArgumentFactories: true)
class ApiResponse<T> {
  final bool success;
  final T? data;
  final String? error;

  ApiResponse({required this.success, this.data, this.error});

  factory ApiResponse.fromJson(
    Map<String, dynamic> json,
    T Function(Object? json) fromJsonT,
  ) => _$ApiResponseFromJson(json, fromJsonT);

  Map<String, dynamic> toJson(Object Function(T value) toJsonT) =>
      _$ApiResponseToJson(this, toJsonT);
}
