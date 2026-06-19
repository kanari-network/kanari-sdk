import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import '../models/auth_models.dart';

/// Kanari Auth API Client for Flutter/Dart applications
///
/// Provides email-based authentication and transaction signing
/// through the run-auth REST API server.
class KanariAuthClient extends ChangeNotifier {
  final String baseUrl;
  final http.Client _client;

  /// Current active session ID (if logged in)
  String? _sessionId;

  /// Current user email (if logged in)
  String? _userEmail;

  /// Current wallet address (if logged in)
  String? _walletAddress;

  /// Check if user is currently authenticated
  bool get isAuthenticated => _sessionId != null;

  /// Get current session ID
  String? get sessionId => _sessionId;

  /// Get current user email
  String? get userEmail => _userEmail;

  /// Get current wallet address
  String? get walletAddress => _walletAddress;

  KanariAuthClient(this.baseUrl, {http.Client? client})
    : _client = client ?? http.Client();

  /// Register a new user account
  ///
  /// [email] - User's email address
  /// [password] - User's password (must meet strength requirements)
  /// [curveType] - Optional cryptographic curve type (ed25519, k256, p256, dilithium2, dilithium3, dilithium5)
  ///
  /// Returns [RegisterResponse] with wallet address on success
  Future<ApiResponse<RegisterResponse>> register({
    required String email,
    required String password,
    String? curveType,
  }) async {
    final request = RegisterRequest(
      email: email,
      password: password,
      curveType: curveType,
    );

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/register'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (response.statusCode == 201) {
      final data = jsonResponse['data'];
      if (data == null) {
        return ApiResponse(
          success: false,
          error: 'No data returned from server',
        );
      }

      final registerResponse = RegisterResponse.fromJson(
        data as Map<String, dynamic>,
      );

      if (registerResponse.walletAddress == null) {
        return ApiResponse(
          success: false,
          error: 'Invalid response: missing wallet address',
        );
      }

      return ApiResponse(success: true, data: registerResponse);
    } else {
      return ApiResponse(
        success: false,
        error: jsonResponse['error'] as String? ?? 'Registration failed',
      );
    }
  }

  /// Login with email and password
  ///
  /// [email] - User's email address
  /// [password] - User's password
  /// [sessionTimeoutHours] - Optional session timeout in hours (default: 24)
  ///
  /// Returns [LoginResponse] with session ID on success
  Future<ApiResponse<LoginResponse>> login({
    required String email,
    required String password,
    String? totpCode,
    String? backupCode,
    int? sessionTimeoutHours,
  }) async {
    final request = LoginRequest(
      email: email,
      password: password,
      totpCode: totpCode,
      backupCode: backupCode,
      sessionTimeoutHours: sessionTimeoutHours,
    );

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/login'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (response.statusCode == 200) {
      final data = jsonResponse['data'];
      if (data == null) {
        return ApiResponse(
          success: false,
          error: 'No data returned from server',
        );
      }

      final loginResponse = LoginResponse.fromJson(
        data as Map<String, dynamic>,
      );

      // Store session information if available
      if (loginResponse.sessionId != null) {
        _sessionId = loginResponse.sessionId;
        _userEmail = loginResponse.userEmail;
        _walletAddress = loginResponse.walletAddress;
        notifyListeners(); // Notify listeners of state change
      }

      return ApiResponse(success: true, data: loginResponse);
    } else {
      return ApiResponse(
        success: false,
        error: jsonResponse['error'] as String? ?? 'Login failed',
      );
    }
  }

  /// Create a pending 2FA setup and return secret, QR, and backup codes.
  Future<ApiResponse<TwoFactorSetupResponse>> setup2fa({
    required String email,
    required String password,
  }) async {
    final request = TwoFactorSetupRequest(email: email, password: password);

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/2fa/setup'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    if (response.statusCode == 200) {
      final data = jsonResponse['data'];
      if (data == null) {
        return ApiResponse(
          success: false,
          error: 'No data returned from server',
        );
      }

      return ApiResponse(
        success: true,
        data: TwoFactorSetupResponse.fromJson(data as Map<String, dynamic>),
      );
    }

    return ApiResponse(
      success: false,
      error: jsonResponse['error'] as String? ?? '2FA setup failed',
    );
  }

  /// Enable 2FA using the setup verification code.
  Future<ApiResponse<Map<String, dynamic>>> enable2fa({
    required String email,
    required String password,
    required String code,
  }) async {
    final request = Enable2faRequest(
      email: email,
      password: password,
      code: code,
    );

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/2fa/enable'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    if (response.statusCode == 200) {
      return ApiResponse(
        success: true,
        data: (jsonResponse['data'] as Map?)?.cast<String, dynamic>(),
      );
    }

    return ApiResponse(
      success: false,
      error: jsonResponse['error'] as String? ?? '2FA enable failed',
    );
  }

  /// Disable 2FA for the given account.
  Future<ApiResponse<Map<String, dynamic>>> disable2fa({
    required String email,
    required String password,
  }) async {
    final request = Disable2faRequest(email: email, password: password);

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/2fa/disable'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    if (response.statusCode == 200) {
      return ApiResponse(
        success: true,
        data: (jsonResponse['data'] as Map?)?.cast<String, dynamic>(),
      );
    }

    return ApiResponse(
      success: false,
      error: jsonResponse['error'] as String? ?? '2FA disable failed',
    );
  }

  /// Verify an already-enabled TOTP code.
  Future<ApiResponse<Map<String, dynamic>>> verify2fa({
    required String email,
    required String code,
  }) async {
    final request = Verify2faRequest(email: email, code: code);

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/2fa/verify'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    if (response.statusCode == 200) {
      return ApiResponse(
        success: true,
        data: (jsonResponse['data'] as Map?)?.cast<String, dynamic>(),
      );
    }

    return ApiResponse(
      success: false,
      error: jsonResponse['error'] as String? ?? '2FA verification failed',
    );
  }

  /// Logout current session
  ///
  /// Returns success message on successful logout
  Future<ApiResponse<Map<String, dynamic>>> logout() async {
    if (_sessionId == null) {
      return ApiResponse(success: false, error: 'No active session');
    }

    final request = LogoutRequest(sessionId: _sessionId!);

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/logout'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (response.statusCode == 200) {
      // Clear session information
      _sessionId = null;
      _userEmail = null;
      _walletAddress = null;
      notifyListeners();

      return ApiResponse(
        success: true,
        data: jsonResponse['data'] as Map<String, dynamic>,
      );
    } else {
      return ApiResponse(
        success: false,
        error: jsonResponse['error'] as String? ?? 'Logout failed',
      );
    }
  }

  /// Logout all sessions for current user
  ///
  /// Returns success message on successful logout
  Future<ApiResponse<Map<String, dynamic>>> logoutAll() async {
    if (_userEmail == null) {
      return ApiResponse(success: false, error: 'No user logged in');
    }

    if (_sessionId == null) {
      return ApiResponse(success: false, error: 'No active session');
    }

    final request = LogoutAllRequest(
      email: _userEmail!,
      sessionId: _sessionId!,
    );

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/logout-all'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (response.statusCode == 200) {
      // Clear session information
      _sessionId = null;
      _userEmail = null;
      _walletAddress = null;
      notifyListeners();

      return ApiResponse(
        success: true,
        data: jsonResponse['data'] as Map<String, dynamic>,
      );
    } else {
      return ApiResponse(
        success: false,
        error: jsonResponse['error'] as String? ?? 'Logout all failed',
      );
    }
  }

  /// Change user password
  ///
  /// [oldPassword] - Current password
  /// [newPassword] - New password (must meet strength requirements)
  ///
  /// Note: This will invalidate all active sessions
  Future<ApiResponse<Map<String, dynamic>>> changePassword({
    required String oldPassword,
    required String newPassword,
  }) async {
    if (_userEmail == null) {
      return ApiResponse(success: false, error: 'No user logged in');
    }

    if (_sessionId == null) {
      return ApiResponse(success: false, error: 'No active session');
    }

    final request = ChangePasswordRequest(
      email: _userEmail!,
      sessionId: _sessionId!,
      oldPassword: oldPassword,
      newPassword: newPassword,
    );

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/change-password'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (response.statusCode == 200) {
      // Clear session information (all sessions invalidated)
      _sessionId = null;
      _userEmail = null;
      _walletAddress = null;
      notifyListeners();

      return ApiResponse(
        success: true,
        data: jsonResponse['data'] as Map<String, dynamic>,
      );
    } else {
      return ApiResponse(
        success: false,
        error: jsonResponse['error'] as String? ?? 'Change password failed',
      );
    }
  }

  /// Delete user account
  ///
  /// [password] - User's password for confirmation
  ///
  /// Warning: This action is irreversible
  Future<ApiResponse<Map<String, dynamic>>> deleteAccount({
    required String password,
  }) async {
    if (_userEmail == null) {
      return ApiResponse(success: false, error: 'No user logged in');
    }

    if (_sessionId == null) {
      return ApiResponse(success: false, error: 'No active session');
    }

    final request = DeleteAccountRequest(
      email: _userEmail!,
      sessionId: _sessionId!,
      password: password,
    );

    final response = await _client.post(
      Uri.parse('$baseUrl/api/v1/delete-account'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(request.toJson()),
    );

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (response.statusCode == 200) {
      // Clear session information
      _sessionId = null;
      _userEmail = null;
      _walletAddress = null;

      return ApiResponse(
        success: true,
        data: jsonResponse['data'] as Map<String, dynamic>,
      );
    } else {
      return ApiResponse(
        success: false,
        error: jsonResponse['error'] as String? ?? 'Delete account failed',
      );
    }
  }

  /// Validate current session
  ///
  /// Returns [ValidateSessionResponse] indicating if session is valid
  Future<ApiResponse<ValidateSessionResponse>> validateSession() async {
    if (_sessionId == null) {
      return ApiResponse(success: false, error: 'No active session');
    }

    try {
      final response = await _client.get(
        Uri.parse('$baseUrl/api/v1/session/validate/$_sessionId'),
      );

      final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

      if (response.statusCode == 200) {
        final data = jsonResponse['data'];
        if (data == null) {
          return ApiResponse(
            success: false,
            error: 'No data returned from server',
          );
        }

        final validateResponse = ValidateSessionResponse.fromJson(
          data as Map<String, dynamic>,
        );

        // If session is invalid, clear local state
        if (!validateResponse.valid) {
          _sessionId = null;
          _userEmail = null;
          _walletAddress = null;
        }

        return ApiResponse(success: true, data: validateResponse);
      } else {
        return ApiResponse(
          success: false,
          error: jsonResponse['error'] as String? ?? 'Validation failed',
        );
      }
    } catch (e) {
      return ApiResponse(success: false, error: 'Validation error: $e');
    }
  }

  /// Manually set session information (for restoring from storage)
  void setSession({
    required String sessionId,
    required String userEmail,
    required String walletAddress,
  }) {
    _sessionId = sessionId;
    _userEmail = userEmail;
    _walletAddress = walletAddress;
    notifyListeners(); // Notify listeners of state change
  }

  /// Clear session information manually
  void clearSession() {
    _sessionId = null;
    _userEmail = null;
    _walletAddress = null;
    notifyListeners(); // Notify listeners of state change
  }
}
