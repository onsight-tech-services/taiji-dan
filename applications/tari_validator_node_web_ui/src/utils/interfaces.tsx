//  Copyright 2022. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

interface IEpoch {
  current_epoch: number;
  is_valid: boolean;
  committee_shard: CommitteeShard | null;
}

interface IIdentity {
  node_id: string;
  public_address: string;
  public_key: string;
}

interface CommitteeShard {
  bucket: number,
  num_committees: number,
  num_members: number,
}


interface IFunction {
  name: string;
  arguments: Array<string>;
  output: string;
}

interface ITemplate {
  registration_metadata: {
    address: string;
    url: string;
    binary_sha: Array<number>;
    height: number;
  };
  abi: { template_name: string; functions: Array<IFunction> };
}

type ICommittees = [string, string, string[]][];

type ICommitteeMap = [number, number, string[]];

interface ICommitteeChart {
  activeleft: number[];
  inactiveleft: number[];
  activemiddle: number[];
  inactiveright: number[];
  activeright: number[];
}

interface GetNetworkCommitteesResponse {
  current_epoch: number;
  committees: Array<CommitteeShardInfo>;
}

interface CommitteeShardInfo {
  bucket: number;
  shard_range: {
    end: string;
    start: string;
  };
  validators: Array<CommitteeValidatorInfo>;
}

interface CommitteeValidatorInfo {
  address: string;
  committee_bucket: number;
  epoch: number;
  shard_key: string;
}

export {
  type IEpoch,
  type IIdentity,
  type ITemplate,
  type ICommittees,
  type ICommitteeChart,
  type ICommitteeMap,
  type GetNetworkCommitteesResponse,
  type CommitteeShardInfo,
  type CommitteeValidatorInfo,
};
