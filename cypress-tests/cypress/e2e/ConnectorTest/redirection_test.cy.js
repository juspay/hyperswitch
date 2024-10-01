function verifyReturnUrl(redirection_url, expected_url, forward_flow) {
    const urlParams = new URLSearchParams(redirection_url.search);
    const paymentStatus = urlParams.get('status');
  
    // Check for valid statuses
    if (paymentStatus !== 'succeeded' && paymentStatus !== 'processing') {
      throw new Error(`Test failed: Unexpected payment status: ${paymentStatus}`);
    }
  
    // Proceed with normal redirection validation
    if (forward_flow) {
      // Handling redirection
      if (redirection_url.host.endsWith(expected_url.host)) {
        // No CORS workaround needed
        cy.window().its("location.origin").should("eq", expected_url.origin);
      } else {
        // Workaround for CORS to allow cross-origin iframe
        cy.origin(
          expected_url.origin,
          { args: { expected_url: expected_url.origin } },
          ({ expected_url }) => {
            cy.window().its("location.origin").should("eq", expected_url);
          }
        );
      }
    }
  }
  
describe('Redirection Flow Tests', () => {
    it('should pass for a URL with succeeded status', () => {
      const redirection_url = 'https://example.com/?payment_id=123&status=succeeded';
      const expected_url = new URL('https://example.com');
      
      cy.visit(redirection_url);
      cy.window().then(() => {
        verifyReturnUrl(new URL(redirection_url), expected_url, true);
      });
    });
  
    it('should fail for a URL with failed status', () => {
      const redirection_url = 'https://example.com/?payment_id=123&status=failed';
      const expected_url = new URL('https://example.com');
      
      cy.visit(redirection_url);
      cy.window().then(() => {
        try {
          verifyReturnUrl(new URL(redirection_url), expected_url, true);
        } catch (e) {
          expect(e.message).to.eq('Test failed: Unexpected payment status: failed');
        }
      });
    });
  });
  