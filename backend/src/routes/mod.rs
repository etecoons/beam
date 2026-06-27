// Beam Backend Routes Module
//
// This module registers and exports all route handlers for the Beam backend api.
// This includes:
// - User Authentication & PIN Verification (auth)
// - File management and explorer operations (files)
// - Chunked and chunkless upload handlers (upload)
//
// Copyright (c) 2026 Beam Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod auth;
pub mod files;
pub mod upload;
pub mod verify_pin;
