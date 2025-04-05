"use strict";
/**
 * Common types used throughout the library
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.CurveType = void 0;
/**
 * Supported elliptic curve types
 */
var CurveType;
(function (CurveType) {
    CurveType["K256"] = "K256";
    CurveType["P256"] = "P256"; // secp256r1/prime256v1
})(CurveType || (exports.CurveType = CurveType = {}));
