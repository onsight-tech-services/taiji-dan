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

import PageHeading from '../../Components/PageHeading';
import Grid from '@mui/material/Grid';
import { StyledPaper } from '../../Components/StyledComponents';
import Accounts from '../Wallet/Components/Accounts';
import Keys from '../Wallet/Components/Keys';
import AccessTokens from '../Wallet/Components/AccessTokens';
import SettingsTabs from './Components/SettingsTabs';
import GeneralSettings from './Components/GeneralSettings';

export interface ISettingsMenu {
  label: string;
  title: string;
  content: React.ReactNode;
}

function SettingsPage() {
  const menuItems = [
    {
      label: 'General',
      title: 'General Settings',
      content: <GeneralSettings />,
    },
    {
      label: 'Accounts',
      title: 'Manage Accounts',
      content: <Accounts />,
    },
    {
      label: 'Keys',
      title: 'Manage Keys',
      content: <Keys />,
    },
    {
      label: 'Access Tokens',
      title: 'Manage Access Tokens',
      content: <AccessTokens />,
    },
  ];

  return (
    <>
      <Grid item xs={12} md={12} lg={12}>
        <PageHeading>Settings</PageHeading>
      </Grid>
      <Grid item xs={12} md={12} lg={12}>
        <StyledPaper>
          <SettingsTabs menuItems={menuItems} />
        </StyledPaper>
      </Grid>
    </>
  );
}

export default SettingsPage;
